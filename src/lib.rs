#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate shrinkwraprs;

pub mod ops;
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
    sync::{atomic::AtomicBool, Arc},
};

use self::systems::DiskSystems;
pub use disk_ops::table::PartitionError;
pub use disk_types;
use ops::luks::LuksParams;
use slotmap::new_key_type;

// TODO: Activate LUKS devices and scan their newly-opened device maps.
// TODO: Support the creation of loopback devices.
// TODO: Fetch active mount points of discovered devices.

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
        /// Marks a device for creating when disk operations are applied.
        const CREATE = 1 << 0;

        /// Marks a device for removal when disk operations are applied.
        const REMOVE = 1 << 1;

        ///
        const CREATE_CHILDREN = 1 << 2;

        /// Devices which support partition tables being created on them.
        ///
        /// Physical disks, DM RAID, and loopback devices support tables.
        const SUPPORTS_TABLE = 1 << 6;

        /// A device which is a child of a LUKS-encrypted partition.
        const LUKS_CHILD = 1 << 7;
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
    #[error(display = "lvm device probing failed")]
    LvmProber(#[error(cause)] LvmProbeError),
    #[error(display = "system execution failed")]
    SystemRun(#[error(cause)] systems::Error),
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
    pub queued_changes: QueuedChanges,
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
    /// Devices that contain children will associate their children here.
    ///
    /// This applies to devices formatted with LVM, LUKS, or that have partition tables.
    pub children: SecondaryMap<DeviceEntity, Vec<DeviceEntity>>,

    /// Every entity in the world has a device, so accesses to this should be infallible.
    pub devices: SecondaryMap<DeviceEntity, Device>,

    /// If a device represents a physical disk, its information is here.
    pub disks: SecondaryMap<DeviceEntity, Disk>,

    /// Devices which exist as device maps
    pub device_maps: SecondaryMap<DeviceEntity, Box<str>>,

    /// Devices which are loopbacks, and their backing file.
    pub loopbacks: SparseSecondaryMap<DeviceEntity, Box<Path>>,

    /// Devices which are encrypted with LUKS
    pub luks: SparseSecondaryMap<DeviceEntity, Option<LuksPassphrase>>,

    /// Devices which are logical volumes of a volume group.
    pub lvs: SparseSecondaryMap<DeviceEntity, (LvmLv, VgEntity)>,

    /// File systems associated with some devices
    ///
    /// These may be disks, partitions, logical volumes, device maps, or loopback devices.
    pub partitions: SparseSecondaryMap<DeviceEntity, Partition>,

    /// Partitions formatted as LVM PVs, which may be assigned to a VG
    pub pvs: SparseSecondaryMap<DeviceEntity, (LvmPv, Option<VgEntity>)>,

    /// Partition tables associated with devices.
    ///
    /// Disk and loopback devices may optionally have these.
    pub tables: SparseSecondaryMap<DeviceEntity, PartitionTable>,
}

/// Stores requested modificactions to an entity.
///
/// This is to prevent overriding existing values which might be cancelled.
/// It also helps to reduce logic required for making changes to the system.
#[derive(Debug, Default)]
pub struct QueuedChanges {
    /// A device to create.
    pub devices: SparseSecondaryMap<DeviceEntity, Device>,

    /// Secured passphrases for LUKS devices.
    pub device_maps: SparseSecondaryMap<DeviceEntity, Box<str>>,

    /// Requests to change a partition's file system.
    pub formats: SparseSecondaryMap<DeviceEntity, FileSystem>,

    /// Requests to change a partition's label.
    pub labels: SparseSecondaryMap<DeviceEntity, Box<str>>,

    /// Options for configuring LUKS encryption
    pub luks: SparseSecondaryMap<DeviceEntity, (DeviceEntity, LuksParams)>,

    /// Information about a device if it is a LVM logical volume.
    pub lvs: SparseSecondaryMap<DeviceEntity, (LvmLv, VgEntity)>,

    /// Devices to be associated if they are successfully created.
    pub parents: SparseSecondaryMap<DeviceEntity, DeviceEntity>,

    /// Devices with parent(s) will associate their parent device(s) here.
    pub partitions: SparseSecondaryMap<DeviceEntity, Partition>,

    /// LVM PVs to associate with volume groups.
    pub pv_parents: SparseSecondaryMap<DeviceEntity, VgEntity>,

    /// LVM devices to be optionally-associated to a volume group
    pub pvs: SparseSecondaryMap<DeviceEntity, (LvmPv, Option<VgEntity>)>,

    /// Volume groups that are to be created, or modified.
    pub volume_groups: SparseSecondaryMap<VgEntity, LvmVg>,

    /// Devices to be associated with a volume group.
    pub vg_parents: SparseSecondaryMap<DeviceEntity, VgEntity>,

    /// Requests to resize a partition.
    pub resize: SparseSecondaryMap<DeviceEntity, (u64, u64)>,

    /// Tables to create
    pub tables: SparseSecondaryMap<DeviceEntity, PartitionTable>,
}

impl QueuedChanges {
    pub fn clear(&mut self) {
        self.devices.clear();
        self.device_maps.clear();
        self.formats.clear();
        self.labels.clear();
        self.luks.clear();
        self.parents.clear();
        self.partitions.clear();
        self.volume_groups.clear();
        self.vg_parents.clear();
        self.resize.clear();
        self.tables.clear();
    }

    /// Removes all entity keys which are associated with a given parent entity.
    pub fn pop_children_of<T: PartialEq>(
        parents: &mut SparseSecondaryMap<DeviceEntity, T>,
        parent: T,
    ) -> impl Iterator<Item = DeviceEntity> + '_ {
        let taker = std::iter::repeat_with(move || {
            match parents.iter().find(|(_, cparent)| **cparent == parent) {
                Some((child, _)) => {
                    parents.remove(child);
                    Some(child)
                }
                None => None,
            }
        });

        taker.take_while(Option::is_some).map(Option::unwrap)
    }
}

impl DiskManager {
    /// Drops all recorded entities and their components.
    pub fn clear(&mut self) { self.entities.clear(); }

    pub fn is_disk(&self, entity: DeviceEntity) -> bool {
        self.components.devices.disks.contains_key(entity)
    }

    pub fn is_partition(&self, entity: DeviceEntity) -> bool {
        self.components.devices.partitions.contains_key(entity)
    }

    pub fn is_luks(&self, entity: DeviceEntity) -> bool {
        self.components.devices.luks.contains_key(entity)
    }

    pub fn is_lvm_lv(&self, entity: DeviceEntity) -> bool {
        self.components.devices.lvs.contains_key(entity)
    }

    pub fn is_lvm_pv(&self, entity: DeviceEntity) -> bool {
        self.components.devices.pvs.contains_key(entity)
    }

    /// Unsets any operations that have been queued.
    pub fn unset(&mut self) {
        self.components.queued_changes.clear();
        self.flags = Default::default();

        let mut entities_to_remove: Vec<DeviceEntity> = Vec::new();
        let mut vg_entities_to_remove: Vec<VgEntity> = Vec::new();

        let entities = &mut self.entities;

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

#[cfg(test)]
mod tests;
