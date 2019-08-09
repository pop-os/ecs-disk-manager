use crate::{device::DeviceExt, sector::SectorExt};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct LvmLv {
    pub name: Box<str>,
    pub path: Box<Path>,
    pub uuid: Box<str>,
}

#[derive(Debug, Clone)]
pub struct LvmPv {
    pub path:       Box<Path>,
    pub uuid:       Box<str>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct LvmVg {
    pub name:         Box<str>,
    pub extent_size:  u64,
    pub extents:      u64,
    pub extents_free: u64,
}

impl LvmVg {
    pub fn extent_size_as_512_byte_sectors(&self) -> u64 {
        assert!(self.extent_size % 512 == 0);
        self.extent_size / 512
    }

    pub fn sectors_free(&self) -> u64 { self.extent_size_as_512_byte_sectors() * self.extents_free }
}

impl DeviceExt for LvmVg {
    fn name(&self) -> &str { &self.name }

    fn path(&self) -> &Path { Path::new("") }

    fn sectors(&self) -> u64 { self.extent_size * self.extents / 512 }

    fn logical_sector_size(&self) -> u64 { 512 }

    fn physical_sector_size(&self) -> u64 { 512 }
}

impl SectorExt for LvmVg {}
