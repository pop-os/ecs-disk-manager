use crate::{
    part_list::PartList,
    topology::Topology,
};
use blkid_sys::*;
use cvt;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    io,
    path::Path,
    ptr,
};
use BlkIdError;

pub struct Probe(blkid_probe);

pub enum ProbeResult {
    Success,
    NothingFound,
    Ambivalent,
}

impl Probe {
    pub fn new() -> Result<Probe, BlkIdError> { unsafe { Ok(Probe(cvt(blkid_new_probe())?)) } }

    pub fn new_from<P: AsRef<Path>>(path: P) -> Result<Probe, BlkIdError> {
        let path = CString::new(path.as_ref().as_os_str().to_string_lossy().as_ref())
            .expect("provided path contained null bytes");

        let probe = unsafe { Probe(cvt(blkid_new_probe_from_filename(path.as_ptr()))?) };

        Ok(probe)
    }

    /// Calls probing functions in all enabled chains. The superblocks chain is enabled by
    /// default. The blkid_do_probe() stores result from only one probing function.
    /// It's necessary to call this routine in a loop to get results from all probing functions
    /// in all chains. The probing is reset by blkid_reset_probe() or by filter functions.
    ///
    /// This is string-based NAME=value interface only.
    ///
    /// Returns `false` on success, and `true` when probing is done.
    pub fn probe(&self) -> Result<bool, BlkIdError> {
        unsafe { cvt(blkid_do_probe(self.0)).map(|v| v == 1) }
    }

    /// This function gathers probing results from all enabled chains and checks for ambivalent
    /// results (e.g. more filesystems on the device).
    ///
    /// This is string-based NAME=value interface only.
    ///
    /// Note about suberblocks chain -- the function does not check for filesystems when a
    /// RAID signature is detected. The function also does not check for collision between RAIDs.
    /// The first detected RAID is returned. The function checks for collision between partition
    /// table and RAID signature -- it's recommended to enable partitions chain together with
    /// superblocks chain.
    /// Returns Ok(0) on success, Ok(1) on success and nothing was detected, Ok(-2) if the probe
    /// was ambivalent.
    pub fn probe_safe(&self) -> Result<ProbeResult, BlkIdError> {
        match unsafe { blkid_do_safeprobe(self.0) } {
            0 => Ok(ProbeResult::Success),
            1 => Ok(ProbeResult::NothingFound),
            -2 => Ok(ProbeResult::Ambivalent),
            _ => Err(BlkIdError::Io(io::Error::last_os_error())),
        }
    }

    /// This function gathers probing results from all enabled chains.
    ///
    /// It is the same as `probe_safe()` but does not check for collision between probing result.
    ///
    /// > This is string-based NAME=value interface only.
    ///
    /// # Errors
    /// Returns either `Err(why)`, `Ok(ProbeResult::Success)`, or `Ok(ProbeResult::NothingFound)`.
    pub fn probe_full(&self) -> Result<ProbeResult, BlkIdError> {
        match unsafe { blkid_do_fullprobe(self.0) } {
            0 => Ok(ProbeResult::Success),
            1 => Ok(ProbeResult::NothingFound),
            _ => Err(BlkIdError::Io(io::Error::last_os_error())),
        }
    }

    // Fetch a value by name.
    pub fn lookup_value<'a>(&'a self, name: &str) -> Result<&'a str, BlkIdError> {
        let name = CString::new(name).expect("provided path contained null bytes");
        let mut data_ptr: *const ::libc::c_char = ptr::null();
        let mut len = 0;
        unsafe {
            cvt::<i32>(blkid_probe_lookup_value(self.0, name.as_ptr(), &mut data_ptr, &mut len))?;
            let data_value = CStr::from_ptr(data_ptr as *const ::libc::c_char);
            data_value.to_str().map_err(|_| BlkIdError::InvalidStr)
        }
    }

    /// Returns `true` if the value exists.
    pub fn has_value(&self, name: &str) -> Result<bool, BlkIdError> {
        let name = CString::new(name).expect("provided path contained null bytes");

        unsafe { cvt(blkid_probe_has_value(self.0, name.as_ptr())).map(|v| v == 1) }
    }

    /// The number of values in probing result
    pub fn numof_values(&self) -> Result<i32, BlkIdError> {
        unsafe { cvt(blkid_probe_numof_values(self.0)) }
    }

    /// Retrieve the Nth item (Name, Value) in the probing result, (0..self.numof_values())
    pub fn get_value(&self, num: i32) -> Result<(String, String), BlkIdError> {
        let mut name_ptr: *const ::libc::c_char = ptr::null();
        let mut data_ptr: *const ::libc::c_char = ptr::null();
        let mut len = 0;

        unsafe {
            cvt(blkid_probe_get_value(self.0, num, &mut name_ptr, &mut data_ptr, &mut len))?;
            let name_value = CStr::from_ptr(name_ptr as *const ::libc::c_char);
            let data_value = CStr::from_ptr(data_ptr as *const ::libc::c_char);
            Ok((
                name_value.to_string_lossy().into_owned(),
                data_value.to_string_lossy().into_owned(),
            ))
        }
    }

    /// Retrieve a HashMap of all the probed values
    pub fn get_values_map(&self) -> Result<HashMap<String, String>, BlkIdError> {
        Ok((0..self.numof_values()?).map(|i| self.get_value(i).expect("'i' is in range")).collect())
    }

    pub fn get_devno(&self) -> u64 { unsafe { blkid_probe_get_devno(self.0) } }

    pub fn get_wholedisk_devno(&self) -> u64 { unsafe { blkid_probe_get_wholedisk_devno(self.0) } }

    pub fn is_wholedisk(&self) -> Result<bool, BlkIdError> {
        unsafe { cvt(blkid_probe_is_wholedisk(self.0)).map(|v| v == 1) }
    }

    pub fn get_size(&self) -> Result<u64, BlkIdError> {
        unsafe { cvt(blkid_probe_get_size(self.0)).map(|s| s as u64) }
    }

    pub fn get_offset(&self) -> Result<u64, BlkIdError> {
        unsafe { cvt(blkid_probe_get_offset(self.0)).map(|s| s as u64) }
    }

    pub fn get_sectorsize(&self) -> u32 { unsafe { blkid_probe_get_sectorsize(self.0) } }

    pub fn get_sectors(&self) -> Result<u64, BlkIdError> {
        unsafe { cvt(blkid_probe_get_sectors(self.0)).map(|s| s as u64) }
    }

    pub fn get_fd(&self) -> Result<i32, BlkIdError> { unsafe { cvt(blkid_probe_get_fd(self.0)) } }

    /// Enables/disables the topology probing for non-binary interface.
    pub fn enable_topology(&self, enable: bool) -> Result<(), BlkIdError> {
        unsafe { cvt(blkid_probe_enable_topology(self.0, enable as i32)).map(|_| ()) }
    }

    /// This is a binary interface for topology values. See also blkid_topology_* functions.
    /// This function is independent on blkid_do_[safe,full]probe() and
    /// blkid_probe_enable_topology() calls.
    /// WARNING: the returned object will be overwritten by the next blkid_probe_get_topology()
    /// call for the same pr. If you want to use more blkid_topopogy objects in the same time you
    /// have to create more blkid_probe handlers (see blkid_new_probe()).
    pub fn get_topology(&self) -> Result<&Topology, BlkIdError> {
        unsafe {
            cvt(blkid_probe_get_topology(self.0))
                .map(|ptr| &*(ptr as *const Topology))
        }
    }

    /// Enables the partitions probing for non-binary interface.
    pub fn enable_partitions(&self, enable: bool) -> Result<&Self, BlkIdError> {
        unsafe {
            cvt(blkid_probe_enable_partitions(self.0, enable as i32))?;
        }

        Ok(self)
    }

    /// Sets probing flags to the partitions prober. This method is optional.
    /// BLKID_PARTS_* flags
    pub fn set_partition_flags(&self, flags: u32) -> Result<&Self, BlkIdError> {
        unsafe {
            cvt(blkid_probe_set_partitions_flags(self.0, flags as i32))?;
        }

        Ok(self)
    }

    /// Enables the superblocks probing for non-binary interface.
    pub fn enable_superblocks(&self, enable: bool) -> Result<&Self, BlkIdError> {
        unsafe {
            cvt(blkid_probe_enable_superblocks(self.0, enable as i32))?;
        }

        Ok(self)
    }

    /// Sets probing flags to the superblocks prober. This method is optional, the default
    /// are BLKID_SUBLKS_DEFAULTS flags.
    /// flags are BLKID_SUBLKS_* flags
    pub fn set_superblock_flags(&self, flags: u32) -> Result<&Self, BlkIdError> {
        unsafe {
            cvt(blkid_probe_set_superblocks_flags(self.0, flags as i32))?;
        }

        Ok(self)
    }

    pub fn get_partitions(&self) -> Result<&PartList, BlkIdError> {
        unsafe { cvt(blkid_probe_get_partitions(self.0)).map(|ptr| &*(ptr as *const _)) }
    }

    pub fn reset(&mut self) { unsafe { blkid_reset_probe(self.0) } }

    pub fn reset_buffers(&mut self) -> Result<i32, BlkIdError> {
        unsafe { cvt(blkid_probe_reset_buffers(self.0)) }
    }

    pub fn hide_range(&mut self, off: u64, len: u64) -> Result<i32, BlkIdError> {
        unsafe { cvt(blkid_probe_hide_range(self.0, off, len)) }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        if self.0.is_null() {
            // No cleanup needed
            return;
        }
        unsafe {
            blkid_free_probe(self.0);
        }
    }
}
