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
    #[error(display = "failed to create LUKS device on {:?}", _0)]
    LuksCreate(Box<Path>, #[error(cause)] ops::luks::Error),
    #[error(display = "attempted to create a device whose parent did not exist")]
    Parentless,
    #[error(display = "failed to add new partition to {:?} partition table on {:?}", _0, _1)]
    TableAdd(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to create {:?} partition table on {:?}", _0, _1)]
    TableCreate(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "partition table is missing on {:?}", _0)]
    TableMissing(Box<Path>),
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write changes to {:?} partition table on {:?}", _0, _1)]
    TableWrite(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to wipe signatures from {:?}", _0)]
    Wipefs(Box<Path>, #[error(cause)] io::Error),
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    let entities = &mut world.entities;
    let queued_changes = &mut world.queued_changes;
    let &mut DeviceComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut partitions,
        ref mut luks,
        ref mut luks_passphrases,
        ..
    } = &mut world.components;

    for (parent_entity, children) in children.iter() {
        let parent_device = &devices[parent_entity];
        if let Some(ref disk) = disks.get(parent_entity) {
            let path = parent_device.path();

            {
                // Check if the disk needs to be wiped and initialized with a new table.
                let parent_flags = &mut entities[parent_entity];
                if parent_flags.contains(EntityFlags::CREATE) {
                    let table = disk.table.ok_or_else(|| Error::TableMissing(path.into()))?;
                    let sector_size = parent_device.logical_sector_size();

                    match table {
                        PartitionTable::Guid => {
                            disk_ops::table::Gpt::create(path, sector_size)
                                .map_err(|why| Error::TableCreate(table, path.into(), why))?
                                .write()
                                .map_err(|why| Error::TableWrite(table, path.into(), why))?;
                        }
                        PartitionTable::Mbr => unimplemented!("mbr tables are not supported"),
                    }

                    *parent_flags -= EntityFlags::CREATE;
                }
            }

            // Then open the disk and begin writing.
            super::open_partitioner(disk, path, |partitioner, table| {
                let partitioner =
                    partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                // Add partitions to the in-memory partition table.
                for &child in children {
                    let child_flags = &entities[child];
                    if child_flags.contains(EntityFlags::CREATE) {
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
                entities[child] -= EntityFlags::CREATE;

                // Take the file system of the newly-created partition, and queue it
                // to be applied in the format system.
                let device = &devices[child];
                match partitions[child].filesystem {
                    Some(FileSystem::Luks) => {
                        let passphrase = queued_changes.luks_passphrases.remove(child);
                        let result = crate::ops::luks::format(
                            device.path.as_ref(),
                            queued_changes
                                .luks_params
                                .remove(child)
                                .as_ref()
                                .expect("creating a luks partition without parameters"),
                            passphrase.as_ref(),
                        );

                        result.map_err(|why| Error::LuksCreate(device.path.clone(), why))?;

                        luks.insert(child, ());

                        if let Some(passphrase) = passphrase {
                            luks_passphrases.insert(child, passphrase);
                        }
                    }
                    Some(fs) => {
                        queued_changes.formats.insert(child, fs);
                    }
                    None => (),
                }
            }
        } else if let Some(partition) = partitions.get(parent_entity) {
            unimplemented!("unsupport device type");
        }
    }

    Ok(())
}
