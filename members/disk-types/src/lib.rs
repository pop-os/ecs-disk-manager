#[macro_use]
extern crate shrinkwraprs;

pub mod fs;
pub mod partitions;
pub mod sector;

pub use crate::{fs::*, partitions::*, sector::*};

use secstr::SecStr;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Disk {
    pub serial: Box<str>,
    pub table:  Option<PartitionTable>,
}

pub trait DeviceExt {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn sectors(&self) -> u64;
    fn logical_sector_size(&self) -> u64;
    fn physical_sector_size(&self) -> u64;
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name:                 Box<str>,
    pub path:                 Box<Path>,
    pub sectors:              u64,
    pub logical_sector_size:  u64,
    pub physical_sector_size: u64,
}

impl DeviceExt for Device {
    fn name(&self) -> &str { &self.name }

    fn path(&self) -> &Path { &self.path }

    fn sectors(&self) -> u64 { self.sectors }

    fn logical_sector_size(&self) -> u64 { self.logical_sector_size }

    fn physical_sector_size(&self) -> u64 { self.physical_sector_size }
}

impl SectorExt for Device {}

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

#[derive(Debug, Clone, Shrinkwrap)]
pub struct LuksPassphrase(SecStr);

impl From<SecStr> for LuksPassphrase {
    fn from(string: SecStr) -> LuksPassphrase { LuksPassphrase(string) }
}
