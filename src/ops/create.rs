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
    Plain(FileSystem),
    /// Create a LUKS device, with an optional passphrase.
    Luks(LuksParams),
}

pub enum CreateOnParent {
    Device { entity: DeviceEntity, start: Sector, end: Sector },
    VolumeGroup { entity: VgEntity, length: u64 },
}

impl DiskManager {
    /// Create a new volume on a device or volume group.
    pub fn create(
        &mut self,
        parent: CreateOnParent,
        label: Box<str>,
        variant: PartitionCreate,
    ) -> Result<(), Error> {
        let (offset, length, logical_sector_size, physical_sector_size, parent) = match parent {
            CreateOnParent::Device { entity, start, end } => {
                let device = &self.components.devices.devices[entity];
                let sectors = self.can_create_on_device(entity, device, start, end)?;

                (
                    sectors.0,
                    sectors.1,
                    device.logical_sector_size,
                    device.physical_sector_size,
                    EntityVariant::Device(entity),
                )
            }
            CreateOnParent::VolumeGroup { entity, length } => {
                self.can_create_on_vg(entity, length)?;

                (0, length, 512, 512, EntityVariant::VolumeGroup(entity))
            }
        };

        // Followed by the partition component.
        let mut partition = Partition { partlabel: Some(label), offset, ..Default::default() };

        // Create a new device entity for the new partition.
        let entity = self.entities.devices.insert(EntityFlags::CREATE);

        // Are you are a LUKS device, or a plain-old-filesystem?
        match variant {
            PartitionCreate::Plain(filesystem) => {
                partition.filesystem = Some(filesystem);
            }
            PartitionCreate::Luks(luks) => {
                partition.filesystem = Some(FileSystem::Luks);
                self.components.queued_changes.luks_params.insert(entity, luks);
            }
        }

        // Then create the device component for the new entity.
        self.components.queued_changes.devices.insert(
            entity,
            Device {
                name: Box::from(""),
                path: Box::from(Path::new("")),
                sectors: length,
                logical_sector_size,
                physical_sector_size,
            },
        );

        // Add the partition component to the entity.
        self.components.queued_changes.partitions.insert(entity, partition);

        // Associate the partition entity with its parent.
        match parent {
            EntityVariant::Device(parent) => {
                self.components.queued_changes.parents.insert(entity, parent);
            }
            EntityVariant::VolumeGroup(parent) => {
                self.components.queued_changes.vg_parents.insert(entity, parent);
            }
        }

        // Remind the manager that the creation system must be run.
        self.flags |= ManagerFlags::CREATE;

        Ok(())
    }

    /// Define that a new partition table will be written to this device.
    pub fn create_table(
        &mut self,
        entity: DeviceEntity,
        kind: PartitionTable,
    ) -> Result<(), Error> {
        self.components.queued_changes.tables.insert(entity, kind);

        self.entities.devices[entity] |= EntityFlags::CREATE;
        self.flags |= ManagerFlags::CREATE;

        // Mark this device to be wiped, and its children freed.
        self.remove(entity);

        Ok(())
    }

    /// Checks if a partition can be inserted into the sectors of this device.
    ///
    /// Returns the sectors where this partition will be created, if it can be created.
    fn can_create_on_device(
        &self,
        parent: DeviceEntity,
        device: &Device,
        start: Sector,
        end: Sector,
    ) -> Result<(u64, u64), Error> {
        let entities = &self.entities.devices;
        match self.components.devices.children.get(parent) {
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
                    if !child_flags.contains(EntityFlags::REMOVE) {
                        let partdev = &self.components.devices.devices[child];
                        let partition = &self.components.devices.partitions[child];

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

    fn can_create_on_vg(&self, parent: VgEntity, length: u64) -> Result<(), Error> {
        let vg = &self.components.vgs.volume_groups[parent];
        if length <= vg.sectors_free() {
            Ok(())
        } else {
            Err(Error::ExceedsDevice)
        }
    }
}
