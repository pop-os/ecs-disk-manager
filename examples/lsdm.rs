use disk_types::*;
use ecs_disk_manager::*;

fn main() {
    let mut manager = DiskManager::default();
    manager.scan().unwrap();

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
            print_partition("  ", partition);
        } else {
            for child in entity.children() {
                println!("  child: {}", child.device().name);
                if let Some(partition) = child.partition() {
                    print_partition("    ", partition);
                }
            }
        }

        for parent in entity.parents() {
            println!("  parent: {}", parent.device().name);
        }
    }
}

fn print_partition(padding: &str, partition: &Partition) {
    println!("{}offset: {}", padding, partition.offset);
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
