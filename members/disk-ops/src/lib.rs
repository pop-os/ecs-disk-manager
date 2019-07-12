#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate cascade;
#[macro_use]
extern crate shrinkwraprs;

use std::{io, path::Path};

pub mod table;

pub mod partition {
    use disk_types::FileSystem;
    use std::{io, path::Path, process::Command};

    pub fn format(device: &Path, fs: FileSystem) -> io::Result<()> {
        let (cmd, args): (&'static str, &'static [&'static str]) = match fs {
            FileSystem::Btrfs => ("mkfs.btrfs", &["-f"]),
            FileSystem::Exfat => ("mkfs.exfat", &[]),
            FileSystem::Ext2 => ("mkfs.ext2", &["-F", "-q"]),
            FileSystem::Ext3 => ("mkfs.ext3", &["-F", "-q"]),
            FileSystem::Ext4 => ("mkfs.ext4", &["-F", "-q", "-E", "lazy_itable_init"]),
            FileSystem::F2fs => ("mkfs.f2fs", &["-q"]),
            FileSystem::Vfat => ("mkfs.fat", &["-F", "32"]),
            FileSystem::Ntfs => ("mkfs.ntfs", &["-FQ", "-q"]),
            FileSystem::Swap => {
                if swap_exists(device) {
                    return Ok(());
                }

                ("mkswap", &["-f"])
            }
            FileSystem::Xfs => ("mkfs.xfs", &["-f"]),
            _ => unimplemented!("creating unsupported file system"),
        };

        let mut cmd = Command::new(cmd);
        cmd.args(args).arg(device);

        eprintln!("creating file system: {:?}", cmd);

        cmd.status()?;

        Ok(())
    }

    pub fn resize(device: &Path, fs: FileSystem) -> io::Result<()> {
        unimplemented!()
    }

    fn swap_exists(path: &Path) -> bool {
        Command::new("swaplabel").arg(path).status().ok().map_or(false, |stat| stat.success())
    }
}
