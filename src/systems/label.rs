use crate::*;
use std::collections::HashMap;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write label")]
    LabelWrite(#[error(cause)] PartitionError),
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let queued_changes = &mut world.queued_changes;
    let &mut DeviceComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut partitions,
        ..
    } = &mut world.components;

    // Store changes that are queued to be applied.
    let mut changed: HashMap<DeviceEntity, Box<str>> = HashMap::new();

    for (parent_entity, children) in children.iter() {
        let parent_device = &devices[parent_entity];
        if let Some(ref disk) = disks.get(parent_entity) {
            let path = parent_device.path();

            // Load the disk's table into memory, in preparation for potential modifications.
            super::open_partitioner(disk, path, |partitioner, table| {
                let partitioner =
                    partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                // Locate the children who have new labels queued.
                for &child in children {
                    if let Some(new_label) = queued_changes.labels.remove(child) {
                        let partition = &partitions[child];
                        partitioner
                            .label(partition.offset + 1, &new_label)
                            .map_err(Error::LabelWrite)?;
                        changed.insert(child, new_label);
                    }
                }

                Ok(())
            })?;

            // Apply the new labels to the in-memory representation.
            for (entity, new_label) in changed.drain() {
                partitions[entity].partlabel = Some(new_label);
            }
        } else {
            unimplemented!("unsupported device type")
        }
    }

    Ok(())
}
