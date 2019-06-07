use crate::fs::FileSystem;

#[derive(Debug, Clone)]
pub struct Partition {
    pub offset:      u64,
    pub length:      u64,
    pub number:      u32,
    pub filesystem:  Option<FileSystem>,
    pub mbr_variant: PartitionType,
}

/// Specifies whether the partition table on the disk is **MSDOS** or **GPT**.
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
pub enum PartitionTable {
    Msdos,
    Gpt,
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
