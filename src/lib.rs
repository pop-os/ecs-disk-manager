#[macro_use]
extern crate err_derive;

mod builder;
mod common;
mod systems;

use disk_prober::{
    slaves_iter, BlockProbeError, BlockProber, DeviceVariant, LvmProbeError, LvmProber,
};
use disk_types::*;
use slotmap::*;
use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub use self::builder::*;
pub use disk_types;
pub use disk_ops::table::PartitionError;
pub use slotmap::DefaultKey as Entity;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct VgEntity(pub(crate) u32);

#[derive(Debug, Error)]
pub enum DiskError {
    #[error(display = "disk operations cancelled by caller")]
    Cancelled,
    #[error(display = "block device probing failed")]
    BlockProber(#[error(cause)] BlockProbeError),
    #[error(display = "device has unknown file system ({})", _0)]
    UnknownFS(Box<str>),
    #[error(display = "lvm device probing failed")]
    LvmProber(#[error(cause)] LvmProbeError),
    #[error(display = "failed to read GUID table from {:?}", _0)]
    NotGuid(Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to move partition ({:?})", _0)]
    Remove(Box<Path>, #[error(cause)] io::Error),
}

#[derive(Debug, Default)]
pub struct DiskManager {
    pub entities: HopSlotMap<Entity, ()>,
    // Components representing current data on devices.
    pub components: DiskComponents,
    // Operations that will be carried out when systems are invoked
    ops: DiskOps,
}

#[derive(Debug, Default)]
pub struct DiskComponents {
    // Components for disk entities

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
    // If the device has a parent, it will be associated here.
    pub parents: SparseSecondaryMap<Entity, Vec<Entity>>,
    // Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SparseSecondaryMap<Entity, Partition>,
    // Devices which map to a LVM VG are associated here.
    pub pvs: SparseSecondaryMap<Entity, (LvmPv, Option<VgEntity>)>,
    // Devices which lie on LVM VGs are marked here.
    pub lvs: SparseSecondaryMap<Entity, (LvmLv, VgEntity)>,

    // Shared storage for multiple associations

    // Stores LVM VG data, referenced by index.
    pub vgs: VolumeGroupShare,
}

#[derive(Debug, Default)]
struct DiskOps {
    pub create:  Vec<(Entity, PartitionBuilder)>,
    pub format:  HashMap<Entity, FileSystem>,
    pub mklabel: HashMap<Entity, PartitionTable>,
    pub remove:  HashMap<Entity, Vec<Entity>>,
}

impl DiskOps {
    fn clear(&mut self) {
        self.create.clear();
        self.format.clear();
        self.mklabel.clear();
        self.remove.clear();
    }
}

impl DiskManager {
    /// Drops all recorded entities and their components.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components.vgs.clear();
    }

    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), DiskError> {
        self.clear();
        systems::scan(self)
    }

    /// Apply all queued disk operations on the system.
    pub fn apply(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), DiskError> {
        let result = systems::apply(self, cancel);
        self.ops.clear();
        result
    }

    // If the device is a loopback, this will kdisplay the backing file.
    pub fn backing_file(&self, entity: Entity) -> Option<&Path> {
        self.components.loopbacks.get(entity).map(AsRef::as_ref)
    }

    // Fetches the children of a device, for devices that have them.
    pub fn children(&self, entity: Entity) -> Option<&[Entity]> {
        self.components.children.get(entity).map(Vec::as_slice)
    }

    /// Fetches a device entity by its entity ID.
    pub fn device(&self, entity: Entity) -> &Device {
        self.components.devices.get(entity).expect("invalid device entity; report this as a bug")
    }

    /// Find a device by its path.
    pub fn device_by_path(&self, path: &Path) -> Option<(Entity, &Device)> {
        self.components
            .devices
            .iter()
            .find(|(_, device)| device.path.as_ref() == path)
    }

    // If the device is a device map, this will return its name.
    pub fn device_map_name(&self, entity: Entity) -> Option<&str> {
        self.components.device_maps.get(entity).map(AsRef::as_ref)
    }

    /// All entities are device entities in the world.
    pub fn devices<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Device)> + 'a {
        self.components.devices.iter()
    }

    /// If the device is a disk, information about that disk can be retrieved here.
    pub fn disk(&self, entity: Entity) -> Option<&Disk> {
        self.components.disks.get(entity)
    }

    /// Some device entities are LUKS crypto devices.
    pub fn crypto_luks<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Luks)> + 'a {
        self.components.luks.iter()
    }

    /// Some device entities are physical disks.
    pub fn disks<'a>(&'a self) -> impl Iterator<Item = (Entity, &'a Disk)> + 'a {
        self.components.disks.iter()
    }

    // If the device is a LUKS partition, information about the LUKS device is here.
    pub fn luks(&self, entity: Entity) -> Option<&Luks> { self.components.luks.get(entity) }

    /// For LV devices which are associated with a VG.
    pub fn lv(&self, entity: Entity) -> Option<(&LvmVg, &LvmLv)> {
        self.components
            .lvs
            .get(entity)
            .map(|(lv, vg_entity)| (self.components.vgs.get(*vg_entity), lv))
    }

    /// Some device entities are logical volumes.
    pub fn lvm_logical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Entity, &'a LvmLv, &'a LvmVg)> + 'a {
        self.components.lvs.iter().map(move |(id, (lvs, vgent))| {
            let vg = self.components.vgs.get(*vgent);
            (id, lvs, vg)
        })
    }

    /// Some device entities are LVM physical volumes.
    pub fn lvm_physical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Entity, &'a LvmPv, Option<&'a LvmVg>)> + 'a {
        self.components.pvs.iter().map(move |(id, (pvs, vgent))| {
            let vg = vgent.map(|vgent| self.components.vgs.get(vgent));
            (id, pvs, vg)
        })
    }

    pub fn lvm_volume_groups(&self) -> impl Iterator<Item = (VgEntity, &LvmVg)> {
        self.components.vgs.iter()
    }

    pub fn lvm_pvs_of_vg(
        &self,
        entity: VgEntity,
    ) -> impl Iterator<Item = (Entity, &LvmPv)> {
        self.components
            .pvs
            .iter()
            .filter(move |(_, (_, pv))| *pv == Some(entity))
            .map(move |(id, (pv, _))| (id, pv))
    }

    pub fn lvm_lvs_of_vg(
        &self,
        entity: VgEntity,
    ) -> impl Iterator<Item = (Entity, &LvmLv)> {
        self.components
            .lvs
            .iter()
            .filter(move |(_, (_, lv))| *lv == entity)
            .map(move |(id, (pv, _))| (id, pv))
    }

    /// Return the parent of this device, if this device has one.
    pub fn parents(&self, entity: Entity) -> Option<&[Entity]> {
        self.components
            .parents
            .get(entity)
            .map(AsRef::as_ref)
    }

    pub fn partition<'b>(&'b self, entity: Entity) -> Option<&'b Partition> {
        self.components.partitions.get(entity)
    }

    /// Some device entities are partitions.
    pub fn partitions<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Entity, &'a Partition)> + 'a {
        self.components
            .partitions
            .iter()
    }



    /// For PVs which may be associated with a VG.
    pub fn pv<'b>(&'b self, entity: Entity) -> Option<(Option<&'b LvmVg>, &'b LvmPv)> {
        self.components.pvs.get(entity).map(|(pv, vg_entity)| {
            let vg = vg_entity.map(|ent| self.components.vgs.get(ent));
            (vg, pv)
        })
    }

    pub fn add(&mut self, device: Entity, info: PartitionBuilder) -> Result<(), DiskError> {
        self.ops.create.push((device, info));
        Ok(())
    }

    pub fn format(&mut self, device: Entity, filesystem: FileSystem) -> Result<(), DiskError> {
        self.ops.format.insert(device, filesystem);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct VolumeGroupShare(Vec<LvmVg>);

impl VolumeGroupShare {
    pub fn clear(&mut self) { self.0.clear(); }

    pub fn insert(&mut self, input: LvmVg) -> VgEntity {
        self.0.push(input);
        VgEntity((self.0.len() - 1) as u32)
    }

    pub fn iter(&self) -> impl Iterator<Item = (VgEntity, &LvmVg)> {
        self.0.iter().enumerate().map(|(id, entity)| (VgEntity(id as u32), entity))
    }

    pub fn get(&self, index: VgEntity) -> &LvmVg { &self.0[index.0 as usize] }
}
