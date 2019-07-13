use crate::*;
use disk_ops::table::{Gpt, Partitioner};
use disk_types::*;

// TODO:
// - Handle parents whom have not been created yet.
// - Mbr partition tables
// - LUKS and LVM devices

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "partition creation system was cancelled")]
    Cancelled,
    #[error(display = "attempted to create a device whose parent did not exist")]
    Parentless,
    #[error(display = "failed to add new partition to {:?} partition table on {:?}", _0, _1)]
    TableAdd(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write changes to {:?} partition table on {:?}", _0, _1)]
    TableWrite(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let entities = &mut world.entities;
    let &mut DiskComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut device_maps,
        ref mut loopbacks,
        ref mut luks,
        ref mut partitions,
        ref mut pvs,
        ref mut lvs,
        ref mut vgs,
    } = &mut world.components;

    for (parent_entity, children) in children.iter() {
        let parent_device = &devices[parent_entity];
        if let Some(ref disk) = disks.get(parent_entity) {
            let path = parent_device.path();
            super::open_partitioner(disk, path, |partitioner, table| {
                let partitioner = partitioner
                    .map_err(|why| Error::TableRead(
                        table,
                        path.into(),
                        why
                    ))?;


                // Add partitions to the in-memory partition table.
                for &child in children {
                    let child_flags = &entities[child];
                    if child_flags.contains(Flags::CREATE) {
                        let child_device = &devices[child];
                        let partition = &mut partitions[child];

                        let start = partition.offset;
                        let end = child_device.sectors + start;
                        let name = partition.partlabel.as_ref().map(AsRef::as_ref);
                        partition.number = partitioner
                            .add(start, end, name)
                            .map_err(|why| Error::TableAdd(table, path.into(), why))?;
                    }
                }

                // Write changes to disk
                partitioner.write().map_err(|why| Error::TableWrite(table, path.into(), why))
            })?;

            // On success, mark the changes as permanent in the world.
            for &child in children {
                entities[child] -= Flags::CREATE;
            }
        } else {
            unimplemented!("unsupport device type");
        }
    }

    Ok(())
}
