use blkid_sys::*;
use std::{ffi::CStr, str};

pub struct Table;

impl Table {
    fn as_ptr(&self) -> blkid_parttable { self as *const _ as *mut _ }

    pub fn get_type(&self) -> &str {
        unsafe {
            let t = blkid_parttable_get_type(self.as_ptr());
            assert!(!t.is_null());
            str::from_utf8_unchecked(CStr::from_ptr(t).to_bytes())
        }
    }
}
