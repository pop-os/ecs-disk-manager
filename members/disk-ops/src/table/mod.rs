mod gpt;

pub use self::gpt::*;

use disk_types::PartitionTable;
use gptman::{GPTPartitionEntry, GPT};
use rand::Rng;
use std::{
    fs::{File, OpenOptions},
    io::{self, Seek, SeekFrom},
    path::Path,
};

pub trait Partitioner {
    /// Adds a new partition to the in-memory partition table.
    fn add(&mut self, start: u64, end: u64, name: Option<&str>) -> PartitionResult<u32>;

    /// Set the label of the partition at the sector.
    fn label(&mut self, sector: u64, label: &str) -> PartitionResult<()>;

    /// The last addressable sector in the table.
    fn last_sector(&self) -> u64;

    /// Removes the partition that resides at the given sector.
    fn remove(&mut self, sector: u64) -> PartitionResult<()>;

    /// Writes the in-memory partition table to the device.
    fn write(&mut self) -> PartitionResult<()>;
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
    TableReload(#[error(cause)] gptman::linux::BlockError),
}

#[derive(Debug, Error)]
pub enum TableError {
    #[error(display = "GPT table error")]
    Gpt(#[error(cause)] gptman::Error),
}

impl From<gptman::Error> for TableError {
    fn from(error: gptman::Error) -> Self { TableError::Gpt(error) }
}

pub type PartitionResult<T> = Result<T, PartitionError>;
