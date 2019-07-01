use crate::*;

pub fn apply(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), DiskError> {
    macro_rules! cancellation_check {
        () => {
            if cancel.load(Ordering::SeqCst) {
                return Err(DiskError::Cancelled);
            }
        };
    }

    let &DiskComponents {
        ref children,
        ref devices,
        ref disks,
        ref device_maps,
        ref loopbacks,
        ref luks,
        ref parents,
        ref partitions,
        ref pvs,
        ref lvs,
        ref vgs,
    } = &world.components;

    // TODO: Determine which operations are safe to carry out in parallel.

//    for &entity in &world.ops.remove {
//        let part = partitions.get(entity).expect("paprtition to be removed is not a partition");
//
//        let parent = parents.get(entity).expect("partition without a parent")[0];
//
//        if let Some(disk) = disks.get(parent) {
//            let table = disk.table.expect("partition on disk without table");
//            let path = world.device(parent).path.as_ref();
//
//            disk_ops::table::delete(path, table, part.offset)
//                .map_err(|why| DiskError::Remove(path.into(), why))?;
//        } else {
//            // TODO: Handle LV removals on VGs
//        }
//
//        cancellation_check!();
//    }
//
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
