///! Method for creating a new partition entities in the world.
use crate::*;
use disk_types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum Error {
    #[error(display = "the new partition exceeds the size of the parent device")]
    ExceedsDevice,
    #[error(display = "the new partition overlaps an existing partition")]
    PartitionOverlap,
    #[error(display = "the end sector lies before the start sector")]
    InputsInverted,
    #[error(display = "parent device is not partitionable")]
    NotPartitionable,
}

impl DiskManager {
    /// Defines to create a new partition on a device, along with any devices
    /// to create from that new partition.
    pub fn create(
        &mut self,
        parent: Entity,
        builder: impl Into<PartitionBuilder>,
    ) -> Result<(), Error> {
        let builder = builder.into();

        // Determine if the partition overlaps, and if not, what its offsets are.
        let (offset, length) = self.can_create(parent, builder.start, builder.end)?;

        // Followed by the partition component.
        let mut partition = Partition { offset, ..Default::default() };

        // Fill in the rest of the components based on the kind of partition being created.
        match builder.kind {
            Some(PartitionVariant::Luks { physical_volume, password, file_system }) => {
                unimplemented!()
            }
            Some(PartitionVariant::Lvm { volume_group, table }) => unimplemented!(),
            Some(PartitionVariant::FileSystem { label, file_system }) => {
                partition.partlabel = label;
                partition.filesystem = Some(file_system);
            }
            None => {}
        }

        // Create a new device entity for the new partition.
        let entity = self.entities.insert(if partition.filesystem.is_some() {
            Flags::CREATE | Flags::FORMAT
        } else {
            Flags::CREATE
        });

        // The partition device will have the same sector sizes as the disk it is created on.
        let sector_sizes = {
            let device = &self.components.devices[parent];
            (device.logical_sector_size, device.physical_sector_size)
        };

        // Then create the device component for the new entity.
        self.components.devices.insert(
            entity,
            Device {
                name: Box::from(""),
                path: Box::from(Path::new("")),
                sectors: length,
                logical_sector_size: sector_sizes.0,
                physical_sector_size: sector_sizes.1,
            },
        );

        // Add the partition component to the entity.
        self.components.partitions.insert(entity, partition);

        // Associate the partition entity with its parent.
        self.components.children[parent].push(entity);

        // Remind the manager that the creation system must be run.
        self.flags |= ManagerFlags::CREATE;

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
