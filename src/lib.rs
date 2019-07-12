#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate shrinkwraprs;

mod builder;
mod ops;
mod systems;

use disk_prober::{
    slaves_iter, BlockProbeError, BlockProber, DeviceVariant, LvmProbeError, LvmProber,
};
use disk_types::*;
use slotmap::*;
use std::{
    collections::HashMap,
    io,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub use self::builder::*;
pub use disk_ops::table::PartitionError;
pub use disk_types;
pub use ops::create::Error as PartitionCreateError;
pub use slotmap::DefaultKey as Entity;

#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub struct VgEntity(pub(crate) u32);

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "block device probing failed")]
    BlockProber(#[error(cause)] BlockProbeError),
    #[error(display = "device has unknown file system ({})", _0)]
    UnknownFS(Box<str>),
    #[error(display = "lvm device probing failed")]
    LvmProber(#[error(cause)] LvmProbeError),
    #[error(display = "failed to read GUID table from {:?}", _0)]
    NotGuid(Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "system execution failed")]
    SystemRun(#[error(cause)] systems::Error),
    #[error(display = "failed to add partition to device")]
    PartitionAdd(#[error(cause)] PartitionCreateError),
}

#[derive(Debug, Default)]
pub struct DiskManager {
    pub entities: HopSlotMap<Entity, Flags>,
    // Components representing current data on devices.
    pub components: DiskComponents,
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
    // pub parents: SparseSecondaryMap<Entity, Vec<Entity>>,
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

impl DiskManager {
    /// Drops all recorded entities and their components.
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), Error> {
        self.clear();
        systems::scan(self)
    }

    /// Apply all queued disk operations on the system.
    pub fn apply(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
        let result = systems::run(self, cancel);
        result.map_err(Error::SystemRun)
    }
}

#[derive(Debug, Default)]
pub struct VolumeGroupShare(Vec<LvmVg>);

impl VolumeGroupShare {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    pub fn insert(&mut self, input: LvmVg) -> VgEntity {
        self.0.push(input);
        VgEntity((self.0.len() - 1) as u32)
    }

    pub fn iter(&self) -> impl Iterator<Item = (VgEntity, &LvmVg)> {
        self.0.iter().enumerate().map(|(id, entity)| (VgEntity(id as u32), entity))
    }

    pub fn get(&self, index: VgEntity) -> &LvmVg {
        &self.0[index.0 as usize]
    }
}

bitflags! {
    pub struct Flags: u8 {
        /// Create a partition or table.
        const CREATE = 1 << 0;
        /// Removes a partition or table.
        const REMOVE = 1 << 1;
        /// Resizes a partition.
        const RESIZE = 1 << 2;
        /// Formats a partition.
        const FORMAT = 1 << 3;
        /// Change the label of the device.
        const LABEL  = 1 << 4;
    }
}
