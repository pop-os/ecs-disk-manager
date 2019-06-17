use ecs_disk_manager::{disk_types::*, *};

fn main() {
    let mut manager = DiskManager::default();

    if let Err(why) = manager.scan() {
        eprintln!("failed to scan devices: {}", why);
        return;
    }

    for (entity, device) in manager.devices() {
        if let Some(disk) = entity.disk() {
            list_disk(&entity, device, disk);
        } else if let Some(dm_name) = entity.device_map_name() {
            list_device_map(&entity, device, dm_name, 0);
        }
    }

    list_by_vg(&manager);
}

fn list_device_map(entity: &DeviceEntity, device: &Device, dm_name: &str, level: usize) {
    let padding = level * 2;
    println!("{1:0$}Device Map: {2}", padding, " ", dm_name);
    println!("{1:0$}  Path:        {2}", padding, " ", device.path.display());
    println!("{1:0$}  Sector Size: {2}", padding, " ", device.logical_sector_size);
    println!("{1:0$}  Sectors:     {2}", padding, " ", device.sectors);

    if let Some((vg, pv)) = entity.pv() {
        println!("{1:0$}  PV:          {2}", padding, " ", pv.path.display());
        println!("{1:0$}  PV UUID:     {2}", padding, " ", pv.uuid);
        if let Some(vg) = vg {
            println!("{1:0$}  VG:          {2}", padding, " ", vg.name);
        }
    } else if let Some((vg, lv)) = entity.lv() {
        println!("{1:0$}  LV:          {2}", padding, " ", lv.name);
        println!("{1:0$}  LV UUID:     {2}", padding, " ", lv.uuid);
        println!("{1:0$}  VG:          {2}", padding, " ", vg.name);
    }

    if let Some(partition) = entity.partition() {
        list_partition(&&entity, partition, level + 1, false);
    }
}

fn list_disk(entity: &DeviceEntity, disk_device: &Device, disk: &Disk) {
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

    if let Some((vg, pv)) = entity.pv() {
        println!("{1:0$}PV:          {2}", padding, " ", pv.path.display());

        if let Some(vg) = vg {
            println!("{1:0$}VG:          {2}", padding, " ", vg.name);
        }
    }

    for child in entity.children() {
        let device = child.device();
        println!("{1:0$}Child:       {2}", padding, " ", device.path.display());
    }

    for parent in entity.parents() {
        let parent = parent.device();
        println!("{1:0$}Parent:      {2}", padding, " ", parent.path.display());
    }
}
