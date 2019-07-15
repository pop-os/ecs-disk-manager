/// ! Method for creating a new partition entities in the world.
use crate::*;
use disk_types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error(display = "the new partition exceeds the size of the parent device")]
    ExceedsDevice,
    #[error(display = "LVM volume group is non-existent")]
    LvmVgNonExistent,
    #[error(display = "the new partition overlaps an existing partition")]
    PartitionOverlap,
    #[error(display = "the end sector lies before the start sector")]
    InputsInverted,
    #[error(display = "parent device is not partitionable")]
    NotPartitionable,
    #[error(display = "cannot create table on device")]
    TablesUnsupported,
}

pub enum PartitionCreate {
    /// Create a simple, plain file system on the partition.
    Plain(Box<str>, FileSystem),
    /// Create a LUKS device, with an optional passphrase.
    Luks(LuksParams),
    /// Create a LVM device that will extend the named LVM volume group.
    Lvm(Box<str>),
}

impl DiskManager {
    /// Defines to create a new partition on a device, along with any devices
    /// to create from that new partition.
    pub fn create(
        &mut self,
        parent: Entity,
        start: Sector,
        end: Sector,
        variant: PartitionCreate,
    ) -> Result<(), Error> {
        // Determine if the partition overlaps, and if not, what its offsets are.
        let (offset, length) = self.can_create(parent, start, end)?;

        // Followed by the partition component.
        let mut partition = Partition { offset, ..Default::default() };

        enum VgItem {
            Existent(VgEntity),
            NonExistent(Box<str>),
        }

        let mut luks_comps = None;
        let mut lvm_comps = None;

        match variant {
            PartitionCreate::Plain(label, filesystem) => {
                partition.partlabel = Some(label);
                partition.filesystem = Some(filesystem);
            }
            PartitionCreate::Luks(luks) => {
                partition.filesystem = Some(FileSystem::Luks);
                luks_comps = Some(luks);
            }
            PartitionCreate::Lvm(group) => {
                partition.filesystem = Some(FileSystem::Lvm);
                let vg = self
                    .lvm_volume_group(&group)
                    .map(|(id, _)| id)
                    .map_or_else(move || VgItem::NonExistent(group), VgItem::Existent);

                lvm_comps = Some(vg);
            }
        }

        // Create a new device entity for the new partition.
        let entity = self.entities.insert(Flags::CREATE);

        // The partition device will have the same sector sizes as the disk it is created on.
        let sector_sizes = {
            let device = &self.components.devices[parent];
            (device.logical_sector_size, device.physical_sector_size)
        };

        // Then create the device component for the new entity.
        self.components.devices.insert(
            entity,
            Device {
                name:                 Box::from(""),
                path:                 Box::from(Path::new("")),
                sectors:              length,
                logical_sector_size:  sector_sizes.0,
                physical_sector_size: sector_sizes.1,
            },
        );

        // Add the partition component to the entity.
        self.components.partitions.insert(entity, partition);

        // Associate the partition entity with its parent.
        self.components.children[parent].push(entity);

        // Assign the lvm components if this is a logical volume.
        if let Some(vg) = lvm_comps {
            self.flags |= ManagerFlags::RELOAD_VGS;

            let vg_entity = match vg {
                VgItem::Existent(entity) => entity,
                VgItem::NonExistent(name) => self.components.vgs.insert(LvmVg {
                    name,
                    extent_size: 4 * 1024 * 1024,
                    extents: 0,
                    extents_free: 0,
                }),
            };

            // Extend the theoretical amount of extents.
            let parent_sector_size = self.components.devices[parent].logical_sector_size;
            let vg = self.components.vgs.get_mut(vg_entity);
            let extents = (parent_sector_size * length) / vg.extent_size;
            vg.extents += extents;
            vg.extents_free += extents;

            let pv = LvmPv { path: Box::from(Path::new("")), uuid: Box::from("") };

            self.components.pvs.insert(entity, (pv, Some(vg_entity)));
        }

        // Assign the luks components if this configures a LUKS PV
        if let Some(luks) = luks_comps {
            self.queued_changes.luks_params.insert(entity, luks);
        }

        // Remind the manager that the creation system must be run.
        self.flags |= ManagerFlags::CREATE;

        Ok(())
    }

    /// Define that a new partition table will be written to this device.
    pub fn create_table(&mut self, entity: Entity, kind: PartitionTable) -> Result<(), Error> {
        let disk = self.components.disks.get_mut(entity).ok_or(Error::TablesUnsupported)?;
        disk.table = Some(kind);

        self.entities[entity] |= Flags::CREATE;
        self.flags |= ManagerFlags::CREATE;

        // Mark this device to be wiped, and its children freed.
        self.remove(entity);

        Ok(())
    }

    /// Checks if a partition can be inserted into the sectors of this device.
    ///
    /// Returns the sectors where this partition will be created, if it can be created.
    fn can_create(&self, parent: Entity, start: Sector, end: Sector) -> Result<(u64, u64), Error> {
        let entities = &self.entities;
        let device = &self.components.devices[parent];
        match self.components.children.get(parent) {
            Some(children) => {
                let offset = device.get_sector(start);
                let end = device.get_sector(end);

                if offset >= end {
                    return Err(Error::InputsInverted);
                }

                if device.sectors < end {
                    return Err(Error::ExceedsDevice);
                }

                // Check if there is any overlap, ignoring children marked for removal.
                for &child in children {
                    let child_flags = entities[child];
                    if !child_flags.contains(Flags::REMOVE) {
                        let partdev = &self.components.devices[child];
                        let partition = &self.components.partitions[child];

                        // The end of the new partition is before the start of the current.
                        let before = || end < partition.offset;
                        // The start of the new partition is after the end of the current.
                        let after = || offset > partition.offset + partdev.sectors;

                        if before() || after() {
                            return Ok((offset, end - offset));
                        } else {
                            return Err(Error::PartitionOverlap);
                        }
                    }
                }

                Ok((offset, end - offset))
            }
            None => Err(Error::NotPartitionable),
        }
    }
}
