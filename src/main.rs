use ecs_disk_manager::*;

fn main() {
    let mut manager = DiskManager::default();
    manager.scan().unwrap();

    for entity in manager.devices() {
        let device = entity.device();
        println!("{}", device.path.display());
        for entity in entity.table() {
            let device = entity.device();
            println!("├─{}:", device.path.display(),);
        }
    }
}
