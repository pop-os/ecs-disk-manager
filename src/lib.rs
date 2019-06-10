mod builder;

use disk_prober::{slaves_iter, DeviceVariant, DiskProberError, Prober};
use disk_types::prelude::*;
use slotmap::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub use self::builder::*;
pub use slotmap::DefaultKey as Entity;

#[derive(Debug)]
pub enum DiskError {
    Cancelled,
    Prober(DiskProberError),
    UnknownFS(Box<str>),
}

#[derive(Debug, Default)]
pub struct DiskManager {
    entities: HopSlotMap<Entity, ()>,
    // Components representing current data on devices.
    storage: DiskComponents,
    // Operations that will be carried out when systems are invoked
    ops: DiskOps,
}

#[derive(Debug, Default)]
struct DiskComponents {
    // Devices that contain children will associate their children here.
    pub children: SparseSecondaryMap<Entity, Vec<Entity>>,
    // Every entity in the world has a device, so accesses to this should be infallable
    pub devices: SparseSecondaryMap<Entity, Device>,
    // If a device represents a physical disk,its information is here.
    pub disks: SparseSecondaryMap<Entity, Disk>,
    // Device maps that were discovered in the system.
    pub device_maps: SparseSecondaryMap<Entity, Box<str>>,
    // Loopback devices will have a backing file associated with them.
    pub loopbacks: SparseSecondaryMap<Entity, Box<Path>>,
    // If a device is a LUKS device, its information is here.
    pub luks: SparseSecondaryMap<Entity, Luks>,
    // If a device is a LVM group, its information is here.
    pub lvms: SparseSecondaryMap<Entity, Lvm>,
    // If the device has a parent, it will be associated here.
    pub parents: SparseSecondaryMap<Entity, Vec<Entity>>,
    // Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SparseSecondaryMap<Entity, Partition>,
}

impl DiskManager {
    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), DiskError> {
        self.entities.clear();
        self.scan_()
    }

    pub fn add(&mut self, device: Entity, info: PartitionBuilder) -> Result<(), DiskError> {
        self.ops.create.push((device, info));
        Ok(())
    }

    pub fn device(&self, entity: Entity) -> &Device {
        self.storage.devices.get(entity).expect("invalid device entity; report this as a bug")
    }

    pub fn devices<'a>(&'a self) -> impl Iterator<Item = DeviceEntity<'a>> + 'a {
        self.entities.keys().map(move |id| DeviceEntity { id, ctx: self })
    }

    pub fn format(&mut self, device: Entity, filesystem: FileSystem) -> Result<(), DiskError> {
        self.ops.format.insert(device, filesystem);
        Ok(())
    }

    pub fn remove(&mut self, device: Entity) -> Result<(), DiskError> {
        self.ops.remove.insert(device);
        Ok(())
    }

    pub fn run(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), DiskError> {
        let result = self.run_(cancel);
        self.ops.clear();
        result
    }

    fn run_(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), DiskError> {
        macro_rules! cancellation_check {
            () => {
                if cancel.load(Ordering::SeqCst) {
                    return Err(DiskError::Cancelled);
                }
            };
        }

        // TODO: Determine which operations are safe to carry out in parallel.

        for entity in &self.ops.remove {
            let device = self.device(*entity);

            // TODO: Remove partition from partition table.

            cancellation_check!();
        }

        for (entity, table) in &self.ops.mklabel {
            let device = self.device(*entity);

            // TODO: Wipefs and mklabel the device.

            cancellation_check!();
        }

        for (entity, fs) in &self.ops.format {
            let device = self.device(*entity);

            // TODO: format the device

            cancellation_check!();
        }

        for (entity, builder) in &self.ops.create {
            let device = self.device(*entity);

            // TODO: create the partitions on this device.

            cancellation_check!();
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn scan_(&mut self) -> Result<(), DiskError> {
        let prober = Prober::new().map_err(DiskError::Prober)?;
        for res in prober.into_iter().filter_map(Result::transpose) {
            let probed = res.map_err(DiskError::Prober)?;
            let info = probed.probe().map_err(DiskError::Prober)?;

            let whole_entity = self.entities.insert(());

            self.storage.devices.insert(
                whole_entity,
                Device {
                    name:                 Box::from(info.device),
                    path:                 Box::from(info.path),
                    sectors:              info.sectors,
                    logical_sector_size:  info.logical_sector_size,
                    physical_sector_size: info.physical_sector_size,
                },
            );

            match info.variant {
                DeviceVariant::Loopback(backing_file) => {
                    self.storage.loopbacks.insert(whole_entity, backing_file);
                }
                DeviceVariant::Map(devmapper) => {
                    self.storage.device_maps.insert(whole_entity, devmapper);
                }
                DeviceVariant::Physical(table) => {
                    self.storage.disks.insert(whole_entity, Disk { serial: String::new(), table });
                }
            }

            if let Some(fstype) = info.fstype {
                self.storage.partitions.insert(
                    whole_entity,
                    Partition {
                        offset:      0,
                        number:      0,
                        filesystem:  fstype.parse().ok(),
                        partuuid:    None,
                        partlabel:   None,
                        mbr_variant: PartitionType::Primary,
                        uuid:        info.uuid,
                    },
                );
            }

            let mut children = Vec::new();
            for partition in info.partitions {
                let part_entity = self.entities.insert(());
                children.push(part_entity);

                self.storage.parents.insert(part_entity, vec![whole_entity]);

                self.storage.devices.insert(
                    part_entity,
                    Device {
                        name:                 partition.device,
                        path:                 partition.path,
                        sectors:              partition.sectors,
                        logical_sector_size:  info.logical_sector_size,
                        physical_sector_size: info.physical_sector_size,
                    },
                );

                self.storage.partitions.insert(
                    part_entity,
                    Partition {
                        offset:      partition.offset,
                        number:      partition.no,
                        filesystem:  partition.fstype.and_then(|fstype| fstype.parse().ok()),
                        partuuid:    partition.partuuid,
                        partlabel:   partition.partlabel,
                        mbr_variant: PartitionType::Primary,
                        uuid:        partition.uuid,
                    },
                );
            }

            self.storage.children.insert(whole_entity, children);
        }

        self.associate_slaves();

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn scan_(&mut self) -> Result<(), DiskError> {
        compile_error!("Only Linux is supported at the moment");
    }

    fn associate_slaves(&mut self) {
        let devices = &self.storage.devices;
        let parents = &mut self.storage.parents;

        for (entity, device) in devices {
            for slave in slaves_iter(&device.name) {
                for (other_entity, other_device) in devices {
                    if other_device.name == slave {
                        match parents.get_mut(entity) {
                            Some(associations) => associations.push(other_entity),
                            None => drop(parents.insert(entity, vec![other_entity])),
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct DiskOps {
    pub create:  Vec<(Entity, PartitionBuilder)>,
    pub format:  HashMap<Entity, FileSystem>,
    pub mklabel: HashMap<Entity, PartitionTable>,
    pub remove:  HashSet<Entity>,
}

impl DiskOps {
    fn clear(&mut self) {
        self.create.clear();
        self.format.clear();
        self.mklabel.clear();
        self.remove.clear();
    }
}

pub struct DeviceEntity<'a> {
    ctx:    &'a DiskManager,
    pub id: Entity,
}

impl<'a> DeviceEntity<'a> {
    // If the device is a loopback, this will kdisplay the backing file.
    pub fn backing_file<'b>(&'b self) -> Option<&'a Path> {
        self.ctx.storage.loopbacks.get(self.id).map(AsRef::as_ref)
    }

    // Provides an iterator for listing children of a device, for devices that support having
    // multiple children.
    pub fn children<'b>(&'b self) -> impl Iterator<Item = DeviceEntity<'b>> {
        self.ctx
            .storage
            .children
            .get(self.id)
            .into_iter()
            .flat_map(|entities| entities.iter())
            .map(move |&id| DeviceEntity { id, ctx: self.ctx })
    }

    // Access information about this device.
    pub fn device<'b>(&'b self) -> &'b Device { self.ctx.device(self.id) }

    // If the device is a device map, this will return its name.
    pub fn device_map_name<'b>(&'b self) -> Option<&'b str> {
        self.ctx.storage.device_maps.get(self.id).map(AsRef::as_ref)
    }

    // If the device is a disk, information about that disk can be retrieved here.
    pub fn disk<'b>(&'b self) -> Option<&'b Disk> { self.ctx.storage.disks.get(self.id) }

    // If the device is part of a LVM group, information about the LVM device is here.
    pub fn lvm<'b>(&'b self) -> Option<&'b Lvm> { self.ctx.storage.lvms.get(self.id) }

    // If the device is a LUKS partition, information about the LUKS device is here.
    pub fn luks<'b>(&'b self) -> Option<&'b Luks> { self.ctx.storage.luks.get(self.id) }

    // Return the parent of this device, if this device has one.
    pub fn parents<'b>(&'b self) -> impl Iterator<Item = DeviceEntity<'b>> {
        self.ctx
            .storage
            .parents
            .get(self.id)
            .into_iter()
            .flat_map(|entities| entities.iter())
            .map(move |&id| DeviceEntity { id, ctx: self.ctx })
    }

    pub fn partition<'b>(&'b self) -> Option<&'a Partition> {
        self.ctx.storage.partitions.get(self.id)
    }
}
