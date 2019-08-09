use super::*;
use crate::*;
use disk_ops::table::{wipe, Gpt, PartitionError, Partitioner};

// TODO:
// - Unmount any entities that are mounted.
// - Deactivate any LUKS / LVM volumes.

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to read {:?} partition table from {:?}", _0, _1)]
    TableRead(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to remove {:?} on {:?} partition table", _1, _0)]
    TableRemove(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to write changes to {:?} partition table on {:?}", _0, _1)]
    TableWrite(PartitionTable, Box<Path>, #[error(cause)] PartitionError),
    #[error(display = "failed to wipe {:?}", _0)]
    Wipefs(Box<Path>, #[error(cause)] io::Error),
}

#[derive(Debug, Default)]
pub struct RemoveSystem;

impl System for RemoveSystem {
    type Err = Error;

    fn run(
        &mut self,
        entities: &mut DiskEntities,
        components: &mut DiskComponents,
        cancel: &AtomicBool,
    ) -> Result<(), Self::Err> {
        let entities = &mut entities.devices;
        let &mut DeviceComponents {
            ref mut children,
            ref mut devices,
            ref mut disks,
            ref mut partitions,
            ref mut tables,
            ..
        } = &mut components.devices;

        // TODO: Remove logical volumes from volume groups.
        // TODO: Error if any devices are mounted
        // TODO: Deactivate LUKS devices
        // TODO: Deactivate LVM volumes
        // TODO: Remove associated device maps from activated LUKS devices.

        fn free_children(
            entities: &mut HopSlotMap<DeviceEntity, EntityFlags>,
            storage: &mut SecondaryMap<DeviceEntity, Vec<DeviceEntity>>,
            parent: DeviceEntity,
        ) {
            let mut freed = Vec::new();
            if let Some(mut children) = storage.remove(parent) {
                while !children.is_empty() {
                    for child in children.drain(..) {
                        if let Some(children) = storage.remove(child) {
                            freed.extend_from_slice(&children);
                        }

                        entities.remove(child);
                    }

                    std::mem::swap(&mut freed, &mut children);
                }
            }
        }

        // Scan for devices and partitions to wipe.
        let mut devices_to_wipe = Vec::new();
        let mut partitions_to_free = HashMap::new();
        for disk_entity in disks.keys() {
            if entities[disk_entity].contains(EntityFlags::REMOVE) {
                devices_to_wipe.push(disk_entity);
            } else if let Some(children) = children.get(disk_entity) {
                let free = children
                    .into_iter()
                    .cloned()
                    .filter(|&entity| entities[entity].contains(EntityFlags::REMOVE))
                    .collect::<Vec<DeviceEntity>>();

                if !free.is_empty() {
                    partitions_to_free.insert(disk_entity, free);
                }
            }
        }

        // Wipe all devices to be wiped.
        for entity in devices_to_wipe {
            let device = &devices[entity];

            wipe(&device.path).map_err(|why| Error::Wipefs(device.path.clone(), why))?;
            partitions.remove(entity);
            free_children(entities, children, entity);

            let flags = &mut entities[entity];
            *flags -= EntityFlags::REMOVE;
            if !flags.contains(EntityFlags::CREATE) {
                tables.remove(entity);
            }
        }

        // Free all partitions from their parent devices.
        for (disk_entity, children_to_free) in partitions_to_free {
            let disk_device = &devices[disk_entity];
            let table = tables[disk_entity];
            let path = disk_device.path();
            super::open_partitioner(table, path, |partitioner, table| {
                let partitioner =
                    partitioner.map_err(|why| Error::TableRead(table, path.into(), why))?;

                for &child in &children_to_free {
                    partitioner.remove(partitions[child].offset + 1).map_err(|why| {
                        let device = &devices[child];
                        Error::TableRemove(table, device.path.clone(), why)
                    })?;
                }

                partitioner.write().map_err(|why| Error::TableWrite(table, path.into(), why))
            })?;

            // On success, free all children from the world.
            for child in children_to_free {
                entities.remove(child);
                free_children(entities, children, child);
            }
        }

        Ok(())
    }
}
