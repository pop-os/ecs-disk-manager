extern crate disk_ops;
use disk_types::{FileSystem, PartitionTable};
use std::path::Path;
use disk_ops::table::{Partitioner, PartitionResult};
use std::io;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("/dev/sdb");
    disk_ops::table::wipe(path)?;

    let mut table = disk_ops::table::Gpt::create(path, 512)?;

    let root_size = 16 * 1024 * 1024 * 1024;

    let efi = table.add(1024, 1024000, "EFI".into())?;
    let root = table.add(1024000, root_size / 512, "Root".into())?;

    table.write()?;

    disk_ops::partition::create(
        Path::new(&format!("/dev/sdb{}", efi)),
        FileSystem::Vfat
    )?;

    disk_ops::partition::create(
        Path::new(&format!("/dev/sdb{}", root)),
        FileSystem::Btrfs
    )?;

    table = disk_ops::table::Gpt::open(path)?;
    table.remove(1024001)?;

    let home = table.add(root_size / 512, table.last_sector(), "Home".into())?;

    table.write()?;

    disk_ops::partition::create(
        Path::new(&format!("/dev/sdb{}", home)),
        FileSystem::Btrfs
    )?;

    Ok(())
}
