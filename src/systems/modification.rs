use super::*;
use crate::*;
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to write label")]
    LabelWrite(#[error(cause)] PartitionError),
    #[error(display = "failed to format {:?} with {}", _0, _1)]
    Mkfs(Box<Path>, FileSystem, #[error(cause)] io::Error),
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
}

#[derive(Debug)]
pub struct ModificationSystem {
    changed: HashMap<DeviceEntity, Box<str>>,
}

impl Default for ModificationSystem {
    fn default() -> Self { Self { changed: HashMap::with_capacity(8) } }
}

impl System for ModificationSystem {
    type Err = Error;

    fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Self::Err> {
        self.changed.clear();

        let entities = &mut entities.devices;
        let queued_changes = &mut components.queued_changes;
        let &mut DeviceComponents {
            ref mut children,
            ref mut devices,
            ref mut disks,
            ref mut partitions,
            ref tables,
            ..
        } = &mut components.devices;

        // TODO: Extend volume groups that already exist.

        for entity in entities.keys() {
            if let Some(fs) = queued_changes.formats.remove(entity) {
                let device = &devices[entity];

                disk_ops::partition::format(device.path.as_ref(), fs)
                    .map_err(|why| Error::Mkfs(device.path.clone(), fs, why))?;

                partitions[entity].filesystem = Some(fs);
            }
        }

        for (parent_entity, children) in children.iter() {
            let parent_device = &devices[parent_entity];
            if let Some(table) = tables.get(parent_entity) {
                let path = parent_device.path();

                // Load the disk's table into memory, in preparation for potential modifications.
                super::open_partitioner(*table, path, |partitioner, table| {
                    let partitioner =
                        partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                    // Locate the children who have new labels queued.
                    for &child in children {
                        if let Some(new_label) = queued_changes.labels.remove(child) {
                            let partition = &partitions[child];
                            partitioner
                                .label(partition.offset + 1, &new_label)
                                .map_err(Error::LabelWrite)?;
                            self.changed.insert(child, new_label);
                        }
                    }

                    Ok(())
                })?;

                // Apply the new labels to the in-memory representation.
                for (entity, new_label) in self.changed.drain() {
                    partitions[entity].partlabel = Some(new_label);
                }
            } else {
                unimplemented!("unsupported device type")
            }
        }

        Ok(())
    }
}
