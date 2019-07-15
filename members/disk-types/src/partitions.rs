use crate::fs::FileSystem;
use core::str::FromStr;

#[derive(Debug, Default, Clone)]
pub struct Partition {
    pub offset:      u64,
    pub number:      u32,
    pub filesystem:  Option<FileSystem>,
    pub partuuid:    Option<Box<str>>,
    pub partlabel:   Option<Box<str>>,
    pub mbr_variant: PartitionType,
    pub uuid:        Option<Box<str>>,
}

/// Specifies whether the partition table on the disk is **MSDOS** or **GPT**.
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum PartitionTable {
    Mbr,
    Guid,
}

impl FromStr for PartitionTable {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let table = match input {
            "dos" | "mbr" => PartitionTable::Mbr,
            "guid" | "gpt" => PartitionTable::Guid,
            _ => return Err(()),
        };

        Ok(table)
    }
}

impl From<PartitionTable> for &'static str {
    fn from(table: PartitionTable) -> Self {
        match table {
            PartitionTable::Mbr => "mbr",
            PartitionTable::Guid => "guid",
        }
    }
}

/// Defines whether the partition is a primary, logical, or extended partition.
///
/// # Note
///
/// This only applies for MBR partition tables.
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum PartitionType {
    Primary,
    Logical,
    Extended,
}

impl Default for PartitionType {
    fn default() -> Self { PartitionType::Primary }
}
