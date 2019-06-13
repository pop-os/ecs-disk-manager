use disk_types::*;
use ecs_disk_manager::*;

fn main() {
    let mut manager = DiskManager::default();

    if let Err(why) = manager.scan() {
        eprintln!("failed to scan devices: {}", why);
        return;
    }

    // list_by_device(&manager);
    list_by_disk(&manager);
    list_by_vg(&manager);
}

fn list_by_disk(manager: &DiskManager) {
    for (entity, disk) in manager.disks() {
        let disk_device = entity.device();

        println!("Disk: {}", disk_device.name);
        println!("  Sector Size: {}", disk_device.logical_sector_size);
        println!("  Sectors:     {}", disk_device.sectors);
        match entity.partition() {
            Some(partition) => print_partition("  ", disk_device, partition),
            None => {
                for child in entity.children() {
                    let child_device = child.device();
                    println!("  child: {}", child_device.name);
                    if let Some(partition) = child.partition() {
                        print_partition("    ", child_device, partition);
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
            let device = lv_entity.device();
            let partition = lv_entity.partition().expect("LV that isn't a partition");

            println!("    Path:        {}", lv.path.display());
            println!("    Sector Size: {}", device.logical_sector_size);
            println!("    Offset:      {}", partition.offset);
            println!("    Length:      {}", device.sectors);
        }
    }
}

fn list_by_device(manager: &DiskManager) {
    for entity in manager.devices() {
        let device = entity.device();
        println!("{}", device.name);
        println!("  path: {}", device.path.display());
        println!("  sectors: {}", device.sectors);
        println!("  logical_sector_size: {}", device.logical_sector_size);
        println!("  physical_sector_size: {}", device.physical_sector_size);

        if let Some(dm_name) = entity.device_map_name() {
            println!("  dm_name: {}", dm_name);
            if let Some((vg, lv)) = entity.lv() {
                println!("  lv: {}", lv.name);
                println!("  vg_parent: {}", vg.name);
                println!("    extent_size: {}", vg.extent_size);
                println!("    extents: {}", vg.extents);
                println!("    extents_free: {}", vg.extents_free);
            } else if let Some((vg, pv)) = entity.pv() {
                println!("  pv: {:?}", pv.path);
                if let Some(vg) = vg {
                    println!("  vg_child: {}", vg.name);
                }
            } else if let Some(luks) = entity.luks() {
                println!("  luks_pv: {}", luks.physical_volume);
            }
        } else if let Some(backing_file) = entity.backing_file() {
            println!("  backing_file: {}", backing_file.display());
        } else if let Some(disk) = entity.disk() {
            if let Some(table) = disk.table {
                println!("  table: {}", <&'static str>::from(table));
            }
        }

        if let Some(partition) = entity.partition() {
            print_partition("  ", device, partition);
        } else {
            for child in entity.children() {
                let child_device = child.device();
                println!("  child: {}", child_device.name);
                if let Some(partition) = child.partition() {
                    print_partition("    ", child_device, partition);
                }
            }
        }

        for parent in entity.parents() {
            println!("  parent: {}", parent.device().name);
        }
    }
}

fn print_partition(padding: &str, partition_device: &Device, partition: &Partition) {
    println!("{}offset: {}", padding, partition.offset);
    println!("{}length: {}", padding, partition_device.sectors);
    println!("{}number: {}", padding, partition.number);

    if let Some(fs) = partition.filesystem {
        println!("{}fs: {}", padding, <&'static str>::from(fs));
    }

    if let Some(uuid) = &partition.uuid {
        println!("{}uuid: {}", padding, uuid);
    }

    if let Some(partuuid) = &partition.partuuid {
        println!("{}partuuid: {}", padding, partuuid);
    }

    if let Some(partlabel) = &partition.partlabel {
        println!("{}partlabel: {}", padding, partlabel);
    }
}
