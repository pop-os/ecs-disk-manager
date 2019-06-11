use cryptsetup::*;
use std::env::args;

fn main() {
    for arg in args().skip(1) {
        let crypt_device = CryptDevice::init_by_name(&arg).unwrap();
        println!("{}", arg);
        println!("       CIPHER: {}", crypt_device.get_cipher().unwrap_or(""));
        println!("  CIPHER_MODE: {}", crypt_device.get_cipher_mode().unwrap_or(""));
        println!("         TYPE: {}", crypt_device.get_type().as_str());
        println!("         UUID: {}", crypt_device.get_uuid());
        println!("  DEVICE_NAME: {}", crypt_device.get_device_name());
        println!(" MDEVICE_NAME: {}", crypt_device.get_metadadata_device_name().unwrap_or(""));
        println!("  DATA_OFFSET: {}", crypt_device.get_data_offset());
        println!("    IV_OFFSET: {}", crypt_device.get_iv_offset());
        println!("    VKEY_SIZE: {}", crypt_device.get_volume_key_size());
        println!("  SECTOR_SIZE: {}", crypt_device.get_sector_size());
    }
}
