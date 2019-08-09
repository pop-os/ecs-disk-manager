#[macro_use]
extern crate shrinkwraprs;

pub mod device;
pub mod fs;
pub mod luks;
pub mod lvm;
pub mod partitions;
pub mod sector;

pub mod disk {
    use crate::partitions::PartitionTable;

    #[derive(Debug, Clone)]
    pub struct Disk {
        pub serial: Box<str>,
    }
}

pub use crate::{device::*, disk::*, fs::*, luks::*, lvm::*, partitions::*, sector::*};
