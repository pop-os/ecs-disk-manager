#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate shrinkwraprs;

mod builder;
pub mod ops;
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
    sync::{atomic::AtomicBool, Arc},
};

pub use self::builder::*;
pub use disk_ops::table::PartitionError;
pub use disk_types;
use ops::luks::LuksParams;
use slotmap::new_key_type;

new_key_type! {
    /// An addressable device in the system, whether it is a physical or logical device.
    pub struct DeviceEntity;
}

new_key_type! {
    /// A LVM volume group, which devices can be associated with, or logically created from.
    pub struct VgEntity;
}

bitflags! {
    pub struct EntityFlags: u8 {
        /// Create a partition or table.
        const CREATE = 1 << 0;
        /// Removes a partition or table.
        const REMOVE = 1 << 1;
    }
}

impl Default for EntityFlags {
    fn default() -> Self { EntityFlags::empty() }
}

bitflags! {
    pub struct ManagerFlags: u8 {
        /// Schedule the creation system to run
        const CREATE = 1 << 0;
        /// Schedule the format system to run
        const FORMAT = 1 << 1;
        /// Schedule the label system to run
        const LABEL = 1 << 2;
        /// Schedule the remove system to run
        const REMOVE = 1 << 3;
        /// Schedule the resize system to run
        const RESIZE = 1 << 4;
        /// Schedules for the VG data to be reloaded.
        const RELOAD_VGS = 1 << 5;
    }
}

impl Default for ManagerFlags {
    fn default() -> Self { ManagerFlags::empty() }
}

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
    PartitionAdd(#[error(cause)] ops::create::Error),
}

#[derive(Debug, Default)]
pub struct DiskManager {
    /// All of the device entities stored in the world, and their associated flags.
    pub entities: HopSlotMap<DeviceEntity, EntityFlags>,

    /// Volume group entities are similar to, but not quite the same as a device.
    pub vg_entities: HopSlotMap<VgEntity, EntityFlags>,

    /// Components representing current data on devices.
    pub components: DeviceComponents,

    /// Components of LVM volume groups
    pub vg_components: VgComponents,

    /// Flags which control the behavior of the manager.
    flags: ManagerFlags,

    /// All queued modifications are stored here.
    queued_changes: QueuedChanges,
}

#[derive(Debug, Default)]
pub struct VgComponents {
    /// Children of the volume group entity.
    pub children: SecondaryMap<VgEntity, Vec<DeviceEntity>>,

    /// Information about the volume group.
    pub volume_groups: SecondaryMap<VgEntity, LvmVg>,
}

#[derive(Debug, Default)]
pub struct DeviceComponents {
    // Components for disk entities
    /// Devices that contain children will associate their children here.
    pub children: SecondaryMap<DeviceEntity, Vec<DeviceEntity>>,

    /// Every entity in the world has a device, so accesses to this should be infallable
    pub devices: SecondaryMap<DeviceEntity, Device>,

    /// If a device represents a physical disk,its information is here.
    pub disks: SecondaryMap<DeviceEntity, Disk>,

    /// Device maps that were discovered in the system.
    pub device_maps: SecondaryMap<DeviceEntity, Box<str>>,

    /// Loopback devices will have a backing file associated with them.
    pub loopbacks: SecondaryMap<DeviceEntity, Box<Path>>,

    /// If a device is a LUKS device, its information is here.
    pub luks: SecondaryMap<DeviceEntity, ()>,

    /// Secured passphrases for LUKS devices.
    ///
    /// Passphrases are secured via [secstr](https://docs.rs/secstr).
    pub luks_passphrases: SecondaryMap<DeviceEntity, LuksPassphrase>,

    /// Information about a device if it is a LVM logical volume.
    pub lvs: SecondaryMap<DeviceEntity, (LvmLv, VgEntity)>,

    /// If the device has a parent, it will be associated here.
    /// pub parents: SecondaryMap<DeviceEntity, Vec<DeviceEntity>>,
    /// Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SecondaryMap<DeviceEntity, Partition>,

    /// LVM devices, and their associated VG parent, is defined here.
    pub pvs: SecondaryMap<DeviceEntity, (LvmPv, Option<VgEntity>)>,
}

/// Stores requested modificactions to an entity.
///
/// This is to prevent overriding existing values which might be cancelled.
#[derive(Debug, Default)]
struct QueuedChanges {
    /// Secured passphrases for LUKS devices.
    device_maps: SecondaryMap<DeviceEntity, Box<str>>,

    /// Requests to change a partition's label.
    labels: SecondaryMap<DeviceEntity, Box<str>>,

    /// Options for configuring LUKS encryption
    luks_params: SecondaryMap<DeviceEntity, LuksParams>,

    /// Secured passphrases for LUKS devices.
    luks_passphrases: SecondaryMap<DeviceEntity, LuksPassphrase>,

    /// Requests to change a partition's file system.
    formats: SecondaryMap<DeviceEntity, FileSystem>,

    /// Requests to resize a partition.
    resize: SecondaryMap<DeviceEntity, (u64, u64)>,
}

impl DiskManager {
    /// Drops all recorded entities and their components.
    pub fn clear(&mut self) { self.entities.clear(); }

    /// Unsets any operations that have been queued.
    pub fn unset(&mut self) {
        self.flags = Default::default();
        let entities = &mut self.entities;
        let mut entities_to_remove: Vec<DeviceEntity> = Vec::new();

        for (entity, flags) in entities.iter_mut() {
            if flags.contains(EntityFlags::CREATE) {
                entities_to_remove.push(entity);
            }
            *flags = Default::default();
        }

        entities_to_remove.into_iter().for_each(|entity| {
            entities.remove(entity);
        });
    }

    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), Error> {
        self.clear();
        systems::scan(self)
    }

    /// Apply all queued disk operations on the system.
    pub fn apply(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
        let result = systems::run(self, cancel);
        self.unset();
        result.map_err(Error::SystemRun)
    }
}
