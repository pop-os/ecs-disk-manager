//! # Device Creation System
//!
//! All operations qhich queue a device to be created will be enacted here. Supported operations in
//! this system are as below, and executed in this order:
//!
//! 1. Creating new partition tables on physical devices
//! 2. Creating new partitions on partition tables
//! 3. Creating new LUKS devices by encryptiong partitions
//!
//! It is important to note that newly-created LUKS partitions will expose a device map as a child
//! device, which will be equal in size to the size of the partition, minus the LUKS header. This
//! device map can be formatted with any file system.

use super::*;
use crate::*;
use disk_ops::table::{Gpt, Partitioner};
use disk_types::*;

use std::path::PathBuf;

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
    pub new_children:               HashMap<DeviceEntity, Vec<DeviceEntity>>,
    pub newly_created_luks_devices: Vec<(DeviceEntity, DeviceEntity, Box<str>)>,
    pub waiting_for_parent:         Vec<DeviceEntity>,
}

impl Default for CreationSystem {
    fn default() -> Self {
        Self {
            new_children:               HashMap::with_capacity(8),
            newly_created_luks_devices: Vec::with_capacity(4),
            waiting_for_parent:         Vec::with_capacity(8),
        }
    }
}

impl System for CreationSystem {
    type Err = Error;

    fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Self::Err> {
        let result = self.run_(entities, components, cancel);

        // Apply all successfully-created children to the world
        for (parent, children) in self.new_children.drain() {
            let children_of = &mut components.devices.children[parent];
            children_of.extend_from_slice(&children);
        }

        // Create all of the child devices for newly-created LUKS devices.
        for (luks_device, child, target_name) in self.newly_created_luks_devices.drain(..) {
            let new_device = {
                let parent = &components.devices.devices[child];
                let path = ["/dev/mapper/", &target_name].concat();
                let header_size = (2 * 1024 * 1024) / parent.logical_sector_size;
                Device {
                    name:                 target_name.clone(),
                    path:                 PathBuf::from(path).into(),
                    sectors:              parent.sectors - header_size,
                    logical_sector_size:  parent.logical_sector_size,
                    physical_sector_size: parent.physical_sector_size,
                }
            };

            components.devices.devices.insert(child, new_device);
            components.devices.device_maps.insert(child, target_name);
            components.devices.children.insert(luks_device, vec![child]);
        }

        result
    }
}

impl CreationSystem {
    fn run_(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        let entities = &mut entities.devices;
        let queued_changes = &mut components.queued_changes;
        let &mut DeviceComponents {
            ref mut children,
            ref mut devices,
            ref mut partitions,
            ref mut luks,
            ref mut tables,
            ..
        } = &mut components.devices;

        // TODO: Create volume groups and their logical volumes.

        // 
        // - Disks with the create flag will be wiped and formatted
        // - Queued partitions will be added to partition tables.
        // - Queued partitions of LVM VGs will be created on the VG as a LV
        // - Queued partitions of LUKS devices will be created on the LUKS device

        // First, the creation of new partition tables.
        for (parent_entity, new_table) in queued_changes.tables.drain() {
            let parent_device = &devices[parent_entity];
            let parent_flags = &mut entities[parent_entity];
            let path = parent_device.path();

            {
                let sector_size = parent_device.logical_sector_size();

                match new_table {
                    PartitionTable::Guid => {
                        disk_ops::table::Gpt::create(path, sector_size)
                            .map_err(|why| Error::TableCreate(new_table, path.into(), why))?
                            .write()
                            .map_err(|why| Error::TableWrite(new_table, path.into(), why))?;
                    }
                    PartitionTable::Mbr => unimplemented!("mbr tables are not supported"),
                }

                tables.insert(parent_entity, new_table);
                *parent_flags -= EntityFlags::CREATE;
            }
        }

        // Second, check if any children need to be created on available partition tables.
        for (parent_entity, &table) in tables.iter() {
            let parent_flags = &mut entities[parent_entity];
            if !parent_flags.contains(EntityFlags::CREATE_CHILDREN) {
                continue;
            }

            *parent_flags -= EntityFlags::CREATE_CHILDREN;
            let parent_device = &devices[parent_entity];
            let path = parent_device.path();
            let mut new_children = Vec::new();

            // Then open the disk and begin writing.
            super::open_partitioner(table, path, |partitioner, table| {
                let partitioner =
                    partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                // Add partitions to the in-memory partition table.
                let queued_children =
                    QueuedChanges::pop_children_of(&mut queued_changes.parents, parent_entity);

                for child in queued_children {
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
                    new_children.push(child);
                }

                // Write changes to disk
                partitioner.write().map_err(|why| Error::TableWrite(table, path.into(), why))
            })?;

            // On success, mark the changes as permanent in the world.
            for &child in &new_children {
                entities[child] -= EntityFlags::CREATE;

                // Take the file system of the newly-created partition, and queue it
                // to be applied in the format system.
                let device = &devices[child];

                match partitions[child].filesystem {
                    Some(FileSystem::Luks) => {
                        let (entity, params) = queued_changes.luks.remove(child).expect(
                            "entities marked for creation with a Luks FS are expected to have \
                             LUKS parameters to use when creating the LUKS device",
                        );

                        let result = crate::ops::luks::format(device.path.as_ref(), &params);
                        result.map_err(|why| Error::LuksCreate(device.path.clone(), why))?;

                        luks.insert(child, params.passphrase);
                        entities[entity] -= EntityFlags::CREATE;
                        self.newly_created_luks_devices.push((child, entity, params.target_name));
                    }
                    Some(fs) => {
                        queued_changes.formats.insert(child, fs);
                    }
                    None => (),
                }
            }
        }

        Ok(())
    }
}
