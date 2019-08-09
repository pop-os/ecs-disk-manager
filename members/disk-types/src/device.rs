use crate::sector::SectorExt;
use std::path::Path;

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
