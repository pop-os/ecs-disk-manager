use blkid_sys::*;

pub struct Topology;

impl Topology {
    fn as_ptr(&self) -> blkid_topology { self as *const _ as *mut _ }

    /// alignment offset in bytes or 0.
    pub fn get_alignment_offset(&self) -> u64 {
        unsafe { blkid_topology_get_alignment_offset(self.as_ptr()) }
    }

    /// minimum io size in bytes or 0.
    pub fn get_minimum_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_minimum_io_size(self.as_ptr()) }
    }

    /// optimal io size in bytes or 0.
    pub fn get_optimal_io_size(&self) -> u64 {
        unsafe { blkid_topology_get_optimal_io_size(self.as_ptr()) }
    }

    /// logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn get_logical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_logical_sector_size(self.as_ptr()) }
    }

    /// logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn get_physical_sector_size(&self) -> u64 {
        unsafe { blkid_topology_get_physical_sector_size(self.as_ptr()) }
    }
}
