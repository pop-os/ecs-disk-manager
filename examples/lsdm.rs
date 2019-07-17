use ecs_disk_manager::{disk_types::*, *};

fn main() {
    let mut manager = DiskManager::default();

    if let Err(why) = manager.scan() {
        eprintln!("failed to scan devices: {}", why);
        return;
    }

    for (entity, device) in manager.devices() {
        if let Some(disk) = manager.disk(entity) {
            list_disk(&manager, entity, device, disk);
        }
    }

    list_by_vg(&manager);
}

fn list_device_map(
    manager: &DiskManager,
    entity: DeviceEntity,
    device: &Device,
    dm_name: &str,
    level: usize,
) {
    let padding = level * 2;
    println!("{1:0$}Device Map: {2}", padding, " ", dm_name);
    println!("{1:0$}  Path:        {2}", padding, " ", device.path.display());
    println!("{1:0$}  Sector Size: {2}", padding, " ", device.logical_sector_size);
    println!("{1:0$}  Sectors:     {2}", padding, " ", device.sectors);

    if let Some((pv, vg)) = manager.pv(entity) {
        println!("{1:0$}  PV:          {2}", padding, " ", pv.path.display());
        println!("{1:0$}  PV UUID:     {2}", padding, " ", pv.uuid);
        if let Some(vg) = vg {
            println!("{1:0$}  VG:          {2}", padding, " ", vg.name);
        }
    } else if let Some((lv, vg)) = manager.lv(entity) {
        let vg = &manager.components.vgs.volume_groups[*vg];
        println!("{1:0$}  LV:          {2}", padding, " ", lv.name);
        println!("{1:0$}  LV UUID:     {2}", padding, " ", lv.uuid);
        println!("{1:0$}  VG:          {2}", padding, " ", vg.name);
    }

    // Finally, check the details of the partition, if the entity is a partition.
    if let Some(partition) = manager.partition(entity) {
        list_partition(manager, entity, partition, level + 1, false);
    }
}

fn list_disk(manager: &DiskManager, entity: DeviceEntity, disk_device: &Device, disk: &Disk) {
    println!("Disk: {}", disk_device.name);
    println!("  Path:        {}", disk_device.path.display());
    println!("  Sector Size: {}", disk_device.logical_sector_size);
    println!("  Sectors:     {}", disk_device.sectors);
    match manager.partition(entity) {
        Some(partition) => list_partition(manager, entity, partition, 1, false),
        None => {
            if let Some(table) = disk.table {
                println!("  Table:       {}", <&'static str>::from(table));
                if let Some(children) = manager.children(entity) {
                    for &child in children {
                        let child_device = manager.device(child);
                        println!("  Child: {}", child_device.name);
                        if let Some(partition) = manager.partition(child) {
                            list_partition(manager, child, partition, 2, true);
                        }
                    }
                }
            }
        }
    }
}

fn list_by_vg(manager: &DiskManager) {
    for (entity, vg) in manager.lvm_volume_groups() {
        println!("VG: {}", vg.name);

        println!("  Extent Size:  {}", vg.extent_size);
        println!("  Extents:      {}", vg.extents);
        println!("  Extents Free: {}", vg.extents_free);
        for (lv_entity, lv) in manager.lvm_lvs_of_vg(entity) {
            let partition = manager.partition(lv_entity).expect("LV that isn't a partition");
            println!("  Child: {}", lv.name);
            list_partition(manager, lv_entity, partition, 2, true);
        }
    }
}

fn list_partition(
    manager: &DiskManager,
    entity: DeviceEntity,
    partition: &Partition,
    level: usize,
    path: bool,
) {
    let padding = level * 2;
    let device = manager.device(entity);

    if path {
        println!("{1:0$}Path:        {2}", padding, " ", device.path.display());
    }
    println!("{1:0$}Sector Size: {2}", padding, " ", device.logical_sector_size);
    println!("{1:0$}Offset:      {2}", padding, " ", partition.offset);
    println!("{1:0$}Length:      {2}", padding, " ", device.sectors);
    println!("{1:0$}Number:      {2}", padding, " ", partition.number);

    if let Some(fs) = partition.filesystem {
        println!("{1:0$}FS:          {2}", padding, " ", <&'static str>::from(fs));
    }

    if let Some(uuid) = &partition.uuid {
        println!("{1:0$}UUID:        {2}", padding, " ", uuid);
    }

    if let Some(partuuid) = &partition.partuuid {
        println!("{1:0$}PartUUID:    {2}", padding, " ", partuuid);
    }

    if let Some(partlabel) = &partition.partlabel {
        println!("{1:0$}PartLabel:   {2}", padding, " ", partlabel);
    }

    if let Some((pv, vg)) = manager.pv(entity) {
        println!("{1:0$}PV:          {2}", padding, " ", pv.path.display());

        if let Some(vg) = vg {
            println!("{1:0$}VG:          {2}", padding, " ", vg.name);
        }
    }

    for parent in manager.parents(entity) {
        let parent = manager.device(parent);
        println!("{1:0$}Parent:      {2}", padding, " ", parent.path.display());
    }

    for &child in manager.children(entity).into_iter().flatten() {
        let device = manager.device(child);
        println!("{1:0$}Child:       {2}", padding, " ", device.path.display());
        if let Some(dm) = manager.device_map_name(child) {
            list_device_map(manager, child, device, dm, level + 1);
        }
    }
}
