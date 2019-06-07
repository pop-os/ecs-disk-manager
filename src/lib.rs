#[macro_use]
extern crate auto_enums;

mod builder;

use disk_types::prelude::*;
use slotmap::*;
use std::{
    collections::{HashMap, HashSet},
    iter,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
};

pub use self::builder::*;
pub use slotmap::DefaultKey as Entity;

#[derive(Debug)]
pub enum DiskError {
    Cancelled
}

#[derive(Debug, Default)]
pub struct DiskManager {
    entities: SlotMap<Entity, ()>,
    // Components representing current data on devices.
    storage: DiskComponents,
    // Operations that will be carried out when systems are invoked
    ops: DiskOps,
}

#[derive(Debug, Default)]
struct DiskComponents {
    // Devices that have children will associate their children here.
    pub children: SparseSecondaryMap<Entity, Entity>,
    // Every entity in the world has a device, so accesses to this should be infallable
    pub devices: SparseSecondaryMap<Entity, Device>,
    // If a device represents a physical disk,its information is here.
    pub disks: SparseSecondaryMap<Entity, Disk>,
    // If a device is a LUKS device, its information is here.
    pub luks: SparseSecondaryMap<Entity, Luks>,
    // If a device is a LVM group, its information is here.
    pub lvms: SparseSecondaryMap<Entity, Lvm>,
    // If the device has a parent, it will be associated here.
    pub parents: SparseSecondaryMap<Entity, Entity>,
    // Devices that are partitions can fetch their partition information here.
    pub partitions: SparseSecondaryMap<Entity, Partition>,
    // Devices with tables will associate their tables here.
    pub tables: SparseSecondaryMap<Entity, Vec<Entity>>,
}

impl DiskManager {
    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), DiskError> {
        self.entities.clear();

        unimplemented!();
    }

    pub fn add(&mut self, device: Entity, info: PartitionBuilder) -> Result<(), DiskError> {
        self.ops.create.push((device, info));
        Ok(())
    }

    pub fn device(&self, entity: Entity) -> &Device {
        self.storage.devices
            .get(entity)
            .expect("invalid device entity; report this as a bug")
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
            }
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
}

#[derive(Debug, Default)]
struct DiskOps {
    pub create: Vec<(Entity, PartitionBuilder)>,
    pub format: HashMap<Entity, FileSystem>,
    pub mklabel: HashMap<Entity, PartitionTable>,
    pub remove: HashSet<Entity>,
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
    // Returns the child of this device, if there is a child.
    pub fn child<'b>(&'b self) -> Option<DeviceEntity<'b>> {
        self.ctx.storage.children.get(self.id).map(move |&id| DeviceEntity { id, ctx: self.ctx })
    }

    // Access information about this device.
    pub fn device<'b>(&'b self) -> &'b Device {
        self.ctx.device(self.id)
    }

    // If the device is a disk, information about that disk can be retrieved here.
    pub fn disk<'b>(&'b self) -> Option<&'b Disk> { self.ctx.storage.disks.get(self.id) }

    #[auto_enum(Iterator)]
    pub fn table<'b>(&'b self) -> impl Iterator<Item = DeviceEntity<'b>> {
        match self.ctx.storage.tables.get(self.id) {
            None => iter::empty(),
            Some(entities) => {
                entities.iter().cloned().map(move |id| DeviceEntity { id, ctx: self.ctx })
            }
        }
    }

    // If the device is part of a LVM group, information about the LVM device is here.
    pub fn lvm<'b>(&'b self) -> Option<&'b Lvm> { self.ctx.storage.lvms.get(self.id) }

    // If the device is a LUKS partition, information about the LUKS device is here.
    pub fn luks<'b>(&'b self) -> Option<&'b Luks> { self.ctx.storage.luks.get(self.id) }

    // Return the parent of this device, if this device has one.
    pub fn parent<'b>(&'b self) -> Option<DeviceEntity<'b>> {
        self.ctx.storage.parents.get(self.id).map(move |&id| DeviceEntity { id, ctx: self.ctx })
    }
}
