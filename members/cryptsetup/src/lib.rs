mod crypt_type;

pub use self::crypt_type::{CryptType, CryptTypeStr};

use cryptsetup_sys::*;
use std::{
    ffi::{CStr, CString},
    ptr,
};

#[derive(Debug)]
pub enum CryptError {
    Init,
    NoSectorSize,
}

pub struct CryptDevice(*mut crypt_device);

impl CryptDevice {
    pub fn init(device: Option<&str>) -> Result<Self, CryptError> {
        let mut pointer = ptr::null_mut();

        unsafe {
            let status = crypt_init(
                &mut pointer,
                match device {
                    Some(device) => as_cstr(device).as_ptr(),
                    None => ptr::null(),
                },
            );

            if status == 0 {
                Ok(Self(pointer))
            } else {
                Err(CryptError::Init)
            }
        }
    }

    pub fn init_by_data_device(device: &str, header: &str) -> Result<Self, CryptError> {
        let mut pointer = ptr::null_mut();

        unsafe {
            let status = crypt_init_data_device(
                &mut pointer,
                as_cstr(device).as_ptr(),
                as_cstr(header).as_ptr(),
            );

            if status == 0 {
                Ok(Self(pointer))
            } else {
                Err(CryptError::Init)
            }
        }
    }

    pub fn init_by_name_and_header(name: &str, header: &str) -> Result<Self, CryptError> {
        let mut pointer = ptr::null_mut();

        unsafe {
            let status = crypt_init_by_name_and_header(
                &mut pointer,
                as_cstr(name).as_ptr(),
                as_cstr(header).as_ptr(),
            );

            if status == 0 {
                Ok(Self(pointer))
            } else {
                Err(CryptError::Init)
            }
        }
    }

    pub fn init_by_name(name: &str) -> Result<Self, CryptError> {
        let mut pointer = ptr::null_mut();

        unsafe {
            let status = crypt_init_by_name(&mut pointer, as_cstr(name).as_ptr());
            if status == 0 {
                Ok(Self(pointer))
            } else {
                Err(CryptError::Init)
            }
        }
    }

    pub fn get_cipher(&self) -> Option<&str> {
        unsafe { ptr_as_opt_str(crypt_get_cipher(self.as_ptr())) }
    }

    pub fn get_cipher_mode(&self) -> Option<&str> {
        unsafe { ptr_as_opt_str(crypt_get_uuid(self.as_ptr())) }
    }

    pub fn get_type(&self) -> CryptTypeStr {
        CryptTypeStr(unsafe { ptr_as_str(crypt_get_type(self.as_ptr())) })
    }

    pub fn get_uuid(&self) -> &str { unsafe { ptr_as_str(crypt_get_uuid(self.as_ptr())) } }

    pub fn get_device_name(&self) -> &str {
        unsafe { ptr_as_str(crypt_get_device_name(self.as_ptr())) }
    }

    pub fn get_metadadata_device_name(&self) -> Option<&str> {
        unsafe { ptr_as_opt_str(crypt_get_metadata_device_name(self.as_ptr())) }
    }

    pub fn get_data_offset(&self) -> u64 { unsafe { crypt_get_data_offset(self.as_ptr()) } }

    pub fn get_iv_offset(&self) -> u64 { unsafe { crypt_get_iv_offset(self.as_ptr()) } }

    pub fn get_volume_key_size(&self) -> u32 {
        unsafe { crypt_get_volume_key_size(self.as_ptr()) as u32 }
    }

    pub fn get_sector_size(&self) -> u32 { unsafe { crypt_get_sector_size(self.as_ptr()) as u32 } }

    pub fn as_ptr(&self) -> *mut crypt_device { self.0 }
}

impl Drop for CryptDevice {
    fn drop(&mut self) { unsafe { crypt_free(self.as_ptr()) } }
}

fn as_cstr(input: &str) -> CString { CString::new(input).unwrap() }

unsafe fn ptr_as_opt_str<'a>(ptr: *const libc::c_char) -> Option<&'a str> {
    if ptr.is_null() {
        None
    } else {
        Some(ptr_as_str(ptr))
    }
}

unsafe fn ptr_as_str<'a>(ptr: *const libc::c_char) -> &'a str {
    CStr::from_ptr(ptr).to_str().expect("cryptsetup returned invalid UTF-8")
}
