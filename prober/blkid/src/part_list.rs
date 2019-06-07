use crate::partition::Partition;
use blkid_sys::*;
use errors::*;
use table::Table;

pub struct PartList;

impl PartList {
    fn as_ptr(&self) -> blkid_partlist { self as *const _ as *mut _ }

    pub fn get_partition(&self, partition: u32) -> Option<&Partition> {
        unsafe {
            let ptr = blkid_partlist_get_partition(self.as_ptr(), partition as libc::c_int);
            if ptr.is_null() {
                None
            } else {
                Some(&*(ptr as *const _))
            }
        }
    }

    pub fn get_partition_by_partno(&self, partition: u32) -> Option<&Partition> {
        unsafe {
            let ptr =
                blkid_partlist_get_partition_by_partno(self.as_ptr(), partition as libc::c_int);
            if ptr.is_null() {
                None
            } else {
                Some(&*(ptr as *const _))
            }
        }
    }

    pub fn get_table(&self) -> Option<&Table> {
        unsafe {
            let table = blkid_partlist_get_table(self.as_ptr());
            if table.is_null() {
                None
            } else {
                Some(&*(table as *const _))
            }
        }
    }

    pub fn numof_partitions(&self) -> Result<u32, BlkIdError> {
        unsafe { cvt(blkid_partlist_numof_partitions(self.as_ptr())).map(|v| v as u32) }
    }
}
