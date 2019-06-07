// Copyright 2017 Red Hat, Inc.

// Licensed under the MIT license <LICENSE or
// http://opensource.org/licenses/MIT> This file may not be copied, modified,
// or distributed except according to those terms.

use std::{ffi::CString, path::Path, ptr};

use crate::errors::*;
use blkid_sys::*;
use dev::Devs;
use BlkIdError;

#[derive(Debug)]
pub struct Cache {
    pub cache: blkid_cache,
}

impl Cache {
    /// Creates a new `Cache` which is stored at the default path.
    pub fn new() -> Result<Cache, BlkIdError> {
        let mut c: blkid_cache = ptr::null_mut();
        unsafe {
            cvt(blkid_get_cache(&mut c, ptr::null()))?;
        }
        Ok(Cache { cache: c })
    }

    /// Creates a new `Cache` which is stored at the given path.
    pub fn new_at<P: AsRef<Path>>(path: P) -> Result<Cache, BlkIdError> {
        let mut c: blkid_cache = ptr::null_mut();
        let path = CString::new(path.as_ref().as_os_str().to_string_lossy().as_ref())
            .expect("provided path contained null bytes");
        unsafe {
            cvt(blkid_get_cache(&mut c, path.as_ptr()))?;
        }
        Ok(Cache { cache: c })
    }

    /// Removes garbage (non-existing devices) from the cache.
    pub fn gc(&self) { unsafe { blkid_gc_cache(self.cache) } }

    /// Probes all block devices.
    pub fn probe_all(&self) -> Result<Devs, BlkIdError> {
        unsafe { cvt(blkid_probe_all(self.cache))? };
        Ok(Devs::new(self))
    }

    /// The libblkid probing is based on devices from /proc/partitions by default. This file
    /// usually does not contain removable devices (e.g. CDROMs) and this kind of devices are
    /// invisible for libblkid.
    ///
    /// This function adds removable block devices to cache (probing is based on information from
    /// the /sys directory). Don't forget that removable devices (floppies, CDROMs, ...) could be
    /// pretty slow. It's very bad idea to call this function by default.
    ///
    /// Note that devices which were detected by this function won't be written to blkid.tab cache
    /// file.
    pub fn probe_all_removable(&self) -> Result<Devs, BlkIdError> {
        unsafe { cvt(blkid_probe_all_removable(self.cache))? };
        Ok(Devs::new(self))
    }

    /// Probes all new block devices.
    pub fn probe_all_new(&self) -> Result<Devs, BlkIdError> {
        unsafe { cvt(blkid_probe_all_new(self.cache))? };
        Ok(Devs::new(self))
    }
}

impl Drop for Cache {
    fn drop(&mut self) { unsafe { blkid_put_cache(self.cache) } }
}
