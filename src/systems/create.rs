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
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write changes to {:?} partition table on {:?}", _0, _1)]
    TableWrite(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to wipe signatures from {:?}", _0)]
    Wipefs(Box<Path>, #[error(cause)] io::Error),
}

#[derive(Debug)]
pub struct CreationSystem {
    pub new_children: HashMap<DeviceEntity, Vec<DeviceEntity>>,
}

impl Default for CreationSystem {
    fn default() -> Self { CreationSystem { new_children: HashMap::with_capacity(8) } }
}

impl CreationSystem {
    pub fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &Arc<AtomicBool>,
    ) -> Result<(), Error> {
        let result = self.run_(entities, components, cancel);

        // Apply all successfully-created children to the world
        for (parent, children) in self.new_children.drain() {
            let children_of = &mut components.devices.children[parent];
            children_of.extend_from_slice(&children);
        }

        result
    }

    fn run_(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &Arc<AtomicBool>,
    ) -> Result<(), Error> {
        let entities = &mut entities.devices;
        let queued_changes = &mut components.queued_changes;
        let &mut DeviceComponents {
            ref mut children,
            ref mut devices,
            ref mut disks,
            ref mut partitions,
            ref mut luks,
            ref mut luks_passphrases,
            ..
        } = &mut components.devices;

        for (parent_entity, children) in children.iter() {
            let parent_device = &devices[parent_entity];
            if let Some(disk) = disks.get_mut(parent_entity) {
                let path = parent_device.path();

                {
                    // Check if the disk needs to be wiped and initialized with a new table.
                    let parent_flags = &mut entities[parent_entity];
                    if parent_flags.contains(EntityFlags::CREATE) {
                        let table = queued_changes
                            .tables
                            .remove(parent_entity)
                            .expect("table was marked for creation, but no table was specified");
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

                        disk.table = Some(table);
                        *parent_flags -= EntityFlags::CREATE;
                    }
                }

                // Then open the disk and begin writing.
                super::open_partitioner(disk, path, |partitioner, table| {
                    let partitioner =
                        partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                    // Add partitions to the in-memory partition table.
                    let new_children = QueuedChanges::pop_children_of_device(
                        &mut queued_changes.parents,
                        parent_entity,
                    );
                    for child in new_children {
                        let child_device = queued_changes
                            .devices
                            .remove(child)
                            .expect("partition is being created without a device component");
                        let mut partition = queued_changes
                            .partitions
                            .remove(child)
                            .expect("partition is being created without a partition component");

                        let start = partition.offset;
                        let end = child_device.sectors + start;
                        let name = partition.partlabel.as_ref().map(AsRef::as_ref);
                        partition.number = partitioner
                            .add(start, end, name)
                            .map_err(|why| Error::TableAdd(table, path.into(), why))?;

                        partitions.insert(child, partition);
                        self.new_children
                            .entry(parent_entity)
                            .and_modify(|vector| vector.push(child))
                            .or_insert_with(|| vec![child]);
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
}
