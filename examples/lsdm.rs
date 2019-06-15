use ecs_disk_manager::{disk_types::*, *};

fn main() {
    let mut manager = DiskManager::default();

    if let Err(why) = manager.scan() {
        eprintln!("failed to scan devices: {}", why);
        return;
    }

    for entity in manager.devices() {
        if let Some(disk) = entity.disk() {
            list_disk(&manager, &entity, disk);
        } else if let Some(dm_name) = entity.device_map_name() {
            list_device_map(&manager, &entity, dm_name);
        }
    }

    list_by_vg(&manager);
}

fn list_device_map(manager: &DiskManager, entity: &DeviceEntity, dm_name: &str) {
    let device = entity.device();
    println!("Device Map: {}", dm_name);
    println!("  Path:        {}", device.path.display());
    println!("  Sector Size: {}", device.logical_sector_size);
    println!("  Sectors:     {}", device.sectors);

    if let Some((vg, pv)) = entity.pv() {
        println!("  PV:          {}", pv.path.display());
        println!("  PV UUID:     {}", pv.uuid);
        if let Some(vg) = vg {
            println!("  VG:          {}", vg.name);
        }
    } else if let Some((vg, lv)) = entity.lv() {
        println!("  LV:          {}", lv.name);
        println!("  LV UUID:     {}", lv.uuid);
        println!("  VG:          {}", vg.name);
    }

    if let Some(partition) = entity.partition() {
        list_partition(&&entity, partition, 1, false);
    }
}

fn list_disk(manager: &DiskManager, entity: &DeviceEntity, disk: &Disk) {
    let disk_device = entity.device();

    println!("Disk: {}", disk_device.name);
    println!("  Path:        {}", disk_device.path.display());
    println!("  Sector Size: {}", disk_device.logical_sector_size);
    println!("  Sectors:     {}", disk_device.sectors);
    match entity.partition() {
        Some(partition) => list_partition(&entity, partition, 1, false),
        None => {
            if let Some(table) = disk.table {
                println!("  Table:       {}", <&'static str>::from(table));
                for child in entity.children() {
                    let child_device = child.device();
                    println!("  Child: {}", child_device.name);
                    if let Some(partition) = child.partition() {
                        list_partition(&child, partition, 2, true);
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
            let partition = lv_entity.partition().expect("LV that isn't a partition");
            println!("  Child: {}", lv.name);
            list_partition(&lv_entity, partition, 2, true);
        }
    }
}

fn list_partition(entity: &DeviceEntity, partition: &Partition, level: usize, path: bool) {
    let padding = level * 2;
    let device = entity.device();

    if path {
        println!("{:padding$}Path:        {}", device.path.display(), padding = padding);
    }
    println!("{:padding$}Sector Size: {}", device.logical_sector_size, padding = padding);
    println!("{:padding$}Offset:      {}", partition.offset, padding = padding);
    println!("{:padding$}Length:      {}", device.sectors, padding = padding);
    println!("{:padding$}Number:      {}", partition.number, padding = padding);

    if let Some(fs) = partition.filesystem {
        println!("{:padding$}FS:          {}", <&'static str>::from(fs), padding = padding);
    }

    if let Some(uuid) = &partition.uuid {
        println!("{:padding$}UUID:        {}", uuid, padding = padding);
    }

    if let Some(partuuid) = &partition.partuuid {
        println!("{:padding$}PartUUID:    {}", partuuid, padding = padding);
    }

    if let Some(partlabel) = &partition.partlabel {
        println!("{:padding$}PartLabel:   {}", partlabel, padding = padding);
    }

    if let Some((vg, pv)) = entity.pv() {
        println!("{:padding$}PV:          {}", pv.path.display(), padding = padding);

        if let Some(vg) = vg {
            println!("{:padding$}VG:          {}", vg.name, padding = padding);
        }
    }

    for child in entity.children() {
        let device = child.device();
        println!("{:padding$}Child:       {}", device.path.display(), padding = padding);
    }

    for parent in entity.parents() {
        let parent = parent.device();
        println!("{:padding$}Parent:      {}", parent.path.display(), padding = padding);
    }
}
