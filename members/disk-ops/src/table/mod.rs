mod gpt;

pub use self::gpt::*;

use disk_types::PartitionTable;
use std::{fs::{File, OpenOptions}, io::{self, Seek, SeekFrom}, path::Path};
use gptman::{GPT, GPTPartitionEntry};
use rand::Rng;

pub trait Partitioner: Sized {
    /// Adds a new partition to the in-memory partition table.
    fn add(&mut self, start: u64, end: u64, name: Option<&str>) -> PartitionResult<u32>;

    /// Creates a new in-memory partition table for the device with the given sector size.
    fn create(device: &Path, sector_size: u64) -> PartitionResult<Self>;

    /// Reads an existing GUID partition table.
    fn open(device: &Path) -> PartitionResult<Self>;

    /// Removes the partition that resides at the given sector.
    fn remove(&mut self, sector: u64) -> PartitionResult<&mut Self>;

    /// The last addressable sector in the table.
    fn last_sector(&self) -> u64;

    /// Writes the in-memory partition table to the device.
    fn write(&mut self) -> PartitionResult<&mut Self>;
}

#[derive(Debug, Error)]
pub enum PartitionError {
    #[error(display = "device could not be opened")]
    DeviceOpen(#[error(cause)] io::Error),
    #[error(display = "device seek failed")]
    DeviceSeek(#[error(cause)] io::Error),
    #[error(display = "device write failed")]
    DeviceWrite(#[error(cause)] TableError),
    #[error(display = "partition limit on device exceeded")]
    LimitExceeded,
    #[error(display = "partition not found")]
    PartitionNotFound,
    #[error(display = "partition could not be removed")]
    PartitionRemove(#[error(cause)] TableError),
    #[error(display = "partition table could not be read")]
    TableRead(#[error(cause)] TableError),
    #[error(display = "partition table could not be reloaded")]
    TableReload(#[error(cause)] io::Error),
}

#[derive(Debug, Error)]
pub enum TableError {
    #[error(display = "GPT table error")]
    Gpt(#[error(cause)] gptman::Error),
}

impl From<gptman::Error> for TableError {
    fn from(error: gptman::Error) -> Self {
        TableError::Gpt(error)
    }
}

pub type PartitionResult<T> = Result<T, PartitionError>;
