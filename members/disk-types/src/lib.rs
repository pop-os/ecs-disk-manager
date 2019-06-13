pub mod fs;
pub mod partitions;
pub mod sector;

pub use crate::{fs::*, partitions::*, sector::*};

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Disk {
    pub serial: Box<str>,
    pub table:  Option<PartitionTable>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name:                 Box<str>,
    pub path:                 Box<Path>,
    pub sectors:              u64,
    pub logical_sector_size:  u64,
    pub physical_sector_size: u64,
}

#[derive(Debug, Clone)]
pub struct LvmVg {
    pub name:         Box<str>,
    pub extent_size:  u64,
    pub extents:      u64,
    pub extents_free: u64,
}

#[derive(Debug, Clone)]
pub struct LvmLv {
    pub name: Box<str>,
    pub path: Box<Path>,
    pub uuid: Box<str>,
}

#[derive(Debug, Clone)]
pub struct LvmPv {
    pub path: Box<Path>,
    pub uuid: Box<str>,
}

#[derive(Debug, Clone)]
pub struct Luks {
    pub physical_volume: Box<str>,
}
