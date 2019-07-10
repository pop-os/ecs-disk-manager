use crate::*;
use disk_ops::table::{Gpt, Partitioner};

pub fn apply(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), DiskError> {
    macro_rules! cancellation_check {
        () => {
            if cancel.load(Ordering::SeqCst) {
                return Err(DiskError::Cancelled);
            }
        };
    }

    let &mut DiskComponents {
        ref mut children,
        ref mut devices,
        ref mut disks,
        ref mut device_maps,
        ref mut loopbacks,
        ref mut luks,
        ref mut parents,
        ref mut partitions,
        ref mut pvs,
        ref mut lvs,
        ref mut vgs,
    } = &mut world.components;

    // TODO: Determine which operations are safe to carry out in parallel.

    for (&dentity, pentities) in &world.ops.remove {
        let mut children = children.get_mut(dentity)
            .expect("device does not have children");



        if let Some(disk) = disks.get(dentity) {
            let mut device = devices.get(dentity)
                .expect("disk entity does not have device");

            let table = disk.table.expect("partition on disk without table");
            let path = device.path.as_ref();

            let mut gpt: Gpt;
            let table: &mut dyn Partitioner = match table {
                PartitionTable::Guid => {
                    gpt = Gpt::open(path)
                        .map_err(|why| DiskError::NotGuid(path.into(), why))?;
                    &mut gpt
                }
                PartitionTable::Mbr => {
                    // TODO: MBR table support.
                    panic!("unimplemented");
                }
            };

            for &pentity in pentities {
                let sector = partitions.get(pentity)
                    .expect("partition entity does not have a device")
                    .offset + 1;

                table.remove(sector);
            }


            table.write();

            // Remove all entity associations.
            for pentity in pentities {
                partitions.remove(*pentity);
                common::remove_item(children, pentity);
                devices.remove(*pentity);
            }
        }

        // TODO: Manage removal of LVs from a VG.

        cancellation_check!();
    }

//    for (entity, &table) in &world.ops.mklabel {
//        let device = world.device(*entity);
//
//        disk_ops::wipe_signatures(device.path.as_ref());
//        disk_ops::table::create(device.path.as_ref(), table);
//
//        cancellation_check!();
//    }
//
//    for (entity, builder) in &world.ops.create {
//        let device = world.device(*entity);
//
//        // TODO: create the partitions and their children here.
//
//        cancellation_check!();
//    }
//
//    for (entity, &fs) in &world.ops.format {
//        let device = world.device(*entity);
//
//        disk_ops::partition::create(device.path.as_ref(), fs);
//
//        cancellation_check!();
//    }

    Ok(())
}
