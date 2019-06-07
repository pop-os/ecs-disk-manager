use super::*;

pub struct Partition;

impl Partition {
    fn as_ptr(&self) -> blkid_partition { self as *const _ as *mut _ }

    // TODO: get_flags();

    /// Returns the optional PartLabel of the partition.
    pub fn get_name(&self) -> Option<&str> {
        unsafe { cstr_to_str(blkid_partition_get_name(self.as_ptr())) }
    }

    pub fn get_partno(&self) -> Result<u32, BlkIdError> {
        unsafe { cvt(blkid_partition_get_partno(self.as_ptr())).map(|v| v as u32) }
    }

    pub fn get_size(&self) -> u64 { unsafe { blkid_partition_get_size(self.as_ptr()) as u64 } }

    pub fn get_start(&self) -> u64 { unsafe { blkid_partition_get_start(self.as_ptr()) as u64 } }

    pub fn get_type(&self) -> i32 { unsafe { blkid_partition_get_type(self.as_ptr()) } }

    pub fn get_type_string(&self) -> Option<&str> {
        unsafe { cstr_to_str(blkid_partition_get_type_string(self.as_ptr())) }
    }

    /// Returns the optional PartUUID of the partition.
    pub fn get_uuid(&self) -> Option<&str> {
        unsafe { cstr_to_str(blkid_partition_get_uuid(self.as_ptr())) }
    }

    pub fn is_extended(&self) -> bool { unsafe { blkid_partition_is_extended(self.as_ptr()) != 0 } }

    pub fn is_logical(&self) -> bool { unsafe { blkid_partition_is_logical(self.as_ptr()) != 0 } }

    pub fn is_primary(&self) -> bool { unsafe { blkid_partition_is_primary(self.as_ptr()) != 0 } }
}
