// Copyright (c) 2017 Chris Holcombe

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

//! See https://www.kernel.org/pub/linux/utils/util-linux/v2.21/libblkid-docs/index.html
//! for the reference manual to the FFI bindings
extern crate blkid_sys;
#[macro_use]
extern crate err_derive;
extern crate libc;

use std::{
    ffi::{CStr, CString},
};

use blkid_sys::*;

pub fn known_fstype(fstype: &str) -> Result<bool, BlkIdError> {
    let fstype = CString::new(fstype).expect("interior null byte in UTF-8 string");

    unsafe { cvt(blkid_known_fstype(fstype.as_ptr())).map(|v| v == 1) }
}

mod cache;
mod dev;
mod errors;
mod part_list;
mod partition;
mod probe;
mod table;
mod tag;
mod topology;

pub use cache::*;
pub use dev::*;
pub use errors::*;
pub use part_list::*;
pub use probe::*;
pub use table::*;
pub use tag::*;
pub use topology::*;

pub(crate) fn cstr_to_str<'a>(value: *const libc::c_char) -> Option<&'a str> {
    if value.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(value).to_str().ok() }
}
