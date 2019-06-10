use ecs_disk_manager::*;

fn main() {
    let mut manager = DiskManager::default();
    manager.scan().unwrap();

    for entity in manager.devices() {
        let device = entity.device();
        println!("{}", device.name);

        if let Some(dm_name) = entity.device_map_name() {
            println!("  dm_name: {}", dm_name);
        } else if let Some(backing_file) = entity.backing_file() {
            println!("  backing_file: {}", backing_file.display());
        } else if let Some(table) = entity.disk() {
            println!("  table: {}", <&'static str>::from(table));
        }

        for child in entity.children() {
            println!("  child: {}", child.device().name);
        }
        for parent in entity.parents() {
            println!("  parent: {}", parent.device().name);
        }
    }
}
