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
use self::systems::DiskSystems;
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

pub enum EntityVariant {
    Device(DeviceEntity),
    VolumeGroup(VgEntity),
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
    /// Entities contained within the world.
    pub entities: DiskEntities,

    /// Components associated with those entities.
    pub components: DiskComponents,

    /// Systems with unique state that the world excecutes.
    systems: DiskSystems,

    /// Flags which control the behavior of the manager.
    flags: ManagerFlags,
}

#[derive(Debug, Default)]
pub struct DiskEntities {
    /// All of the device entities stored in the world, and their associated flags.
    pub devices: HopSlotMap<DeviceEntity, EntityFlags>,

    /// Volume group entities are similar to, but not quite the same as a device.
    pub vgs: HopSlotMap<VgEntity, EntityFlags>,
}

impl DiskEntities {
    pub fn clear(&mut self) {
        self.devices.clear();
        self.vgs.clear();
    }
}

#[derive(Debug, Default)]
pub struct DiskComponents {
    /// Components representing current data on devices.
    pub devices: DeviceComponents,

    /// Components of LVM volume groups
    pub vgs: VgComponents,

    /// All queued component modifications are stored here.
    pub(crate) queued_changes: QueuedChanges,
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

    /// Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SecondaryMap<DeviceEntity, Partition>,

    /// LVM devices, and their associated VG parent, is defined here.
    pub pvs: SecondaryMap<DeviceEntity, (LvmPv, Option<VgEntity>)>,
}

/// Stores requested modificactions to an entity.
///
/// This is to prevent overriding existing values which might be cancelled.
/// It also helps to reduce logic required for making changes to the system.
#[derive(Debug, Default)]
struct QueuedChanges {
    /// A device to create.
    pub devices: SecondaryMap<DeviceEntity, Device>,

    /// Secured passphrases for LUKS devices.
    pub device_maps: SecondaryMap<DeviceEntity, Box<str>>,

    /// Requests to change a partition's file system.
    pub formats: SecondaryMap<DeviceEntity, FileSystem>,

    /// Requests to change a partition's label.
    pub labels: SecondaryMap<DeviceEntity, Box<str>>,

    /// Options for configuring LUKS encryption
    pub luks_params: SecondaryMap<DeviceEntity, LuksParams>,

    /// Secured passphrases for LUKS devices.
    pub luks_passphrases: SecondaryMap<DeviceEntity, LuksPassphrase>,

    /// Devices to be associated if they are successfully created.
    pub parents: SecondaryMap<DeviceEntity, DeviceEntity>,

    /// Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SecondaryMap<DeviceEntity, Partition>,

    /// Devices to be associated with a volume group.
    pub vg_parents: SecondaryMap<DeviceEntity, VgEntity>,

    /// Requests to resize a partition.
    pub resize: SecondaryMap<DeviceEntity, (u64, u64)>,

    /// Tables to create
    pub tables: SecondaryMap<DeviceEntity, PartitionTable>,
}

impl QueuedChanges {
    pub fn pop_children_of_device(
        parents: &mut SecondaryMap<DeviceEntity, DeviceEntity>,
        parent: DeviceEntity,
    ) -> impl Iterator<Item = DeviceEntity> + '_ {
        parents.iter().filter(move |(child, cparent)| **cparent == parent).map(|(child, _)| child)
    }

    pub fn pop_children_of_vg(
        parents: &mut SecondaryMap<DeviceEntity, VgEntity>,
        parent: VgEntity,
    ) -> impl Iterator<Item = DeviceEntity> + '_ {
        parents.iter().filter(move |(child, cparent)| **cparent == parent).map(|(child, _)| child)
    }
}

impl DiskManager {
    /// Drops all recorded entities and their components.
    pub fn clear(&mut self) { self.entities.clear(); }

    /// Unsets any operations that have been queued.
    pub fn unset(&mut self) {
        self.flags = Default::default();
        let entities = &mut self.entities;
        let mut entities_to_remove: Vec<DeviceEntity> = Vec::new();
        let mut vg_entities_to_remove: Vec<VgEntity> = Vec::new();

        for (entity, flags) in entities.devices.iter_mut() {
            if flags.contains(EntityFlags::CREATE) {
                entities_to_remove.push(entity);
            }
            *flags = Default::default();
        }

        entities_to_remove.into_iter().for_each(|entity| {
            entities.devices.remove(entity);
        });

        for (entity, flags) in entities.vgs.iter_mut() {
            if flags.contains(EntityFlags::CREATE) {
                vg_entities_to_remove.push(entity);
            }
            *flags = Default::default();
        }

        vg_entities_to_remove.into_iter().for_each(|entity| {
            entities.vgs.remove(entity);
        });
    }

    /// Reloads all disk information from the system.
    pub fn scan(&mut self) -> Result<(), Error> {
        self.clear();
        let &mut DiskManager { ref mut entities, ref mut components, .. } = self;
        systems::scan(entities, components)
    }

    /// Apply all queued disk operations on the system.
    pub fn apply(&mut self, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
        let result = {
            let &mut DiskManager {
                ref mut entities,
                ref mut components,
                ref mut systems,
                ref flags,
            } = self;
            systems::run(entities, components, systems, flags, cancel)
        };

        self.unset();
        result.map_err(Error::SystemRun)
    }
}
