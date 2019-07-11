use crate::*;
use disk_ops::table::{Gpt, PartitionError, Partitioner, wipe};

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

    // TODO: Unmount any entities that are mounted.
    // TODO: Deactivate any LUKS / LVM volumes.
    //
    fn free_children(
        entities: &mut HopSlotMap<Entity, Flags>,
        storage: &mut SparseSecondaryMap<Entity, Vec<Entity>>,
        parent: Entity
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
        if entities[disk_entity].contains(Flags::REMOVE) {
            devices_to_wipe.push(disk_entity);
        } else if let Some(children) = children.get(disk_entity) {
            let free = children.into_iter()
                .cloned()
                .filter(|&entity| entities[entity].contains(Flags::REMOVE))
                .collect::<Vec<Entity>>();

            if !free.is_empty() {
                partitions_to_free.insert(disk_entity, free);
            }
        }
    }

    // Wipe all devices to be wiped.
    for entity in devices_to_wipe {
        let device = &devices[entity];
        wipe(&device.path).map_err(|why| Error::Wipefs(device.path.clone(), why))?;

        entities.remove(entity);
        free_children(entities, children, entity);
    }

    // Free all partitions from their parent devices.
    for (disk_entity, children_to_free) in partitions_to_free {
        let disk_device = &devices[disk_entity];
        let disk = &disks[disk_entity];
        let table = disk.table.expect("partitions are being removed from a disk without a table");
        let path = disk_device.path.as_ref();

        // Temporary variables for storing could-be table values.
        let mut gpt: Gpt;

        // Fetch a generic partitioner depending on the table kind.
        let partitioner: &mut dyn Partitioner = match table {
            PartitionTable::Guid => {
                gpt = Gpt::open(path)
                    .map_err(|why| Error::TableRead(table, path.into(), why))?;
                &mut gpt
            }
            PartitionTable::Mbr => {
                // TODO: MBR table support.
                panic!("unimplemented");
            }
        };

        for &child in &children_to_free {
            partitioner.remove(partitions[child].offset + 1)
                .map_err(|why| {
                    let device = &devices[child];
                    Error::TableRemove(table, device.path.clone(), why)
                })?;
        }

        partitioner.write().map_err(|why| Error::TableWrite(table, path.into(), why))?;

        // On success, free all children from the world.
        for child in children_to_free {
            entities.remove(child);
            free_children(entities, children, child);
        }
    }

    Ok(())
}
