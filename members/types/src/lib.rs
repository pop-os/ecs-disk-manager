pub mod fs;
pub mod partitions;
pub mod sector;

pub mod prelude {
    pub use crate::{fs::*, partitions::*, sector::*, *};
}

use secstr::SecStr;
use std::path::Path;

use crate::partitions::PartitionTable;

#[derive(Debug)]
pub struct Disk {
    pub serial: String,
    pub table:  Option<PartitionTable>,
}

#[derive(Debug)]
pub struct Device {
    pub name:                 Box<str>,
    pub path:                 Box<Path>,
    pub sectors:              u64,
    pub logical_sector_size:  u64,
    pub physical_sector_size: u64,
}

#[derive(Debug)]
pub struct Lvm {
    pub volume_group: String,
}

#[derive(Debug)]
pub struct Luks {
    pub physical_volume: String,
    pub passphrase:      Option<SecStr>,
}
