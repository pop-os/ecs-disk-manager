#[macro_use]
extern crate err_derive;

mod builder;
mod entity;
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

pub use self::{builder::*, entity::*};
pub use disk_types;
pub use slotmap::DefaultKey as Entity;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct VgEntity(pub(crate) u32);

#[derive(Debug, Error)]
pub enum DiskError {
    #[error(display = "disk operations cancelled by caller")]
    Cancelled,
    #[error(display = "block device probing failed: {}", _0)]
    BlockProber(BlockProbeError),
    #[error(display = "device has unknown file system: {}", _0)]
    UnknownFS(Box<str>),
    #[error(display = "lvm device probing failed: {}", _0)]
    LvmProber(LvmProbeError),
    #[error(display = "failed to move partition ({:?}): {}", _0, _1)]
    Remove(Box<Path>, io::Error),
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

    /// Fetches a device entity by its entity ID.
    pub fn device(&self, entity: Entity) -> &Device {
        self.components.devices.get(entity).expect("invalid device entity; report this as a bug")
    }

    /// Find a device by its path.
    pub fn device_by_path(&self, path: &Path) -> Option<(DeviceEntity, &Device)> {
        self.components
            .devices
            .iter()
            .find(|(_, device)| device.path.as_ref() == path)
            .map(move |(id, device)| (DeviceEntity { id, ctx: self }, device))
    }

    /// All entities are device entities in the world.
    pub fn devices<'a>(&'a self) -> impl Iterator<Item = DeviceEntity<'a>> + 'a {
        self.entities.keys().map(move |id| DeviceEntity { id, ctx: self })
    }

    /// Some device entities are LUKS crypto devices.
    pub fn crypto_luks<'a>(&'a self) -> impl Iterator<Item = (DeviceEntity<'a>, &'a Luks)> + 'a {
        self.components.luks.iter().map(move |(id, luks)| (DeviceEntity { id, ctx: self }, luks))
    }

    /// Some device entities are physical disks.
    pub fn disks<'a>(&'a self) -> impl Iterator<Item = (DeviceEntity<'a>, &'a Disk)> + 'a {
        self.components.disks.iter().map(move |(id, disk)| (DeviceEntity { id, ctx: self }, disk))
    }

    /// Some device entities are logical volumes.
    pub fn lvm_logical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (DeviceEntity<'a>, &'a LvmLv, &'a LvmVg)> + 'a {
        self.components.lvs.iter().map(move |(id, (lvs, vgent))| {
            let vg = self.components.vgs.get(*vgent);
            (DeviceEntity { id, ctx: self }, lvs, vg)
        })
    }

    /// Some device entities are LVM physical volumes.
    pub fn lvm_physical_volumes<'a>(
        &'a self,
    ) -> impl Iterator<Item = (DeviceEntity<'a>, &'a LvmPv, Option<&'a LvmVg>)> + 'a {
        self.components.pvs.iter().map(move |(id, (pvs, vgent))| {
            let vg = vgent.map(|vgent| self.components.vgs.get(vgent));
            (DeviceEntity { id, ctx: self }, pvs, vg)
        })
    }

    pub fn lvm_volume_groups<'a>(&'a self) -> impl Iterator<Item = (VgEntity, &'a LvmVg)> {
        self.components.vgs.iter()
    }

    pub fn lvm_pvs_of_vg<'a>(
        &'a self,
        entity: VgEntity,
    ) -> impl Iterator<Item = (DeviceEntity<'a>, &'a LvmPv)> {
        self.components
            .pvs
            .iter()
            .filter(move |(_, (_, pv))| *pv == Some(entity))
            .map(move |(id, (pv, _))| (DeviceEntity { id, ctx: self }, pv))
    }

    pub fn lvm_lvs_of_vg<'a>(
        &'a self,
        entity: VgEntity,
    ) -> impl Iterator<Item = (DeviceEntity<'a>, &'a LvmLv)> {
        self.components
            .lvs
            .iter()
            .filter(move |(_, (_, lv))| *lv == entity)
            .map(move |(id, (pv, _))| (DeviceEntity { id, ctx: self }, pv))
    }

    /// Some device entities are partitions.
    pub fn partitions<'a>(
        &'a self,
    ) -> impl Iterator<Item = (DeviceEntity<'a>, &'a Partition)> + 'a {
        self.components
            .partitions
            .iter()
            .map(move |(id, part)| (DeviceEntity { id, ctx: self }, part))
    }

    pub fn add(&mut self, device: Entity, info: PartitionBuilder) -> Result<(), DiskError> {
        self.ops.create.push((device, info));
        Ok(())
    }

    pub fn format(&mut self, device: Entity, filesystem: FileSystem) -> Result<(), DiskError> {
        self.ops.format.insert(device, filesystem);
        Ok(())
    }

    pub fn remove(&mut self, device: Entity) -> Result<(), DiskError> {
        self.ops.remove.insert(device);
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
