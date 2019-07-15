use disk_types::{FileSystem, Sector};
use secstr::SecStr;

/// Abstraction which provides a builder pattern for constructing new partitions.
///
/// Supports the creation of standard, LVM, and LUKS partitions. LVM and LUKS
/// partitions constructed here may also construct their own partitions in a
/// recursive manner.
#[derive(Clone, Debug)]
pub struct PartitionBuilder {
    pub(crate) start: Sector,
    pub(crate) end:   Sector,
    pub(crate) kind:  Option<PartitionVariant>,
}

impl PartitionBuilder {
    pub fn new(start: Sector, end: Sector) -> Self { PartitionBuilder { start, end, kind: None } }

    /// Specifies which type of partition to create.
    pub fn variant(mut self, variant: impl Into<PartitionVariant>) -> Self {
        self.kind = Some(variant.into());
        self
    }
}

/// Defines the type of partition to create, whether it is a LUKS, LVM, or standard file system.
#[derive(Clone, Debug)]
pub enum PartitionVariant {
    Luks {
        physical_volume: Box<str>,
        password:        Option<SecStr>,
        file_system:     Option<Box<PartitionVariant>>,
    },
    Lvm {
        volume_group: Box<str>,
        table:        Vec<(Box<str>, PartitionBuilder)>,
    },
    FileSystem {
        label:       Option<Box<str>>,
        file_system: FileSystem,
    },
}

/// Abstraction for creating the LVM `PartitionVariant`.
pub struct LvmBuilder {
    volume_group: Box<str>,
    table:        Vec<(Box<str>, PartitionBuilder)>,
}

impl LvmBuilder {
    /// Constructs a new LVM volume with the given volume group.
    ///
    /// If the volume group already exists, this will be assigned to it.
    pub fn new(volume_group: Box<str>) -> Self { Self { volume_group, table: Vec::new() } }

    /// Adds a partition to this volume group, which will be given a LV name.
    ///
    /// - Logical volumes must have a defined logical volume name.
    /// - Names are not allowed to collide.
    pub fn partition(mut self, partition: impl Into<PartitionBuilder>, name: Box<str>) -> Self {
        self.table.push((name, partition.into()));
        self
    }
}

impl From<LvmBuilder> for PartitionVariant {
    fn from(builder: LvmBuilder) -> PartitionVariant {
        PartitionVariant::Lvm { volume_group: builder.volume_group, table: builder.table }
    }
}

pub struct LuksBuilder {
    physical_volume: Box<str>,
    password:        Option<SecStr>,
    file_system:     Option<Box<PartitionVariant>>,
}

impl LuksBuilder {
    pub fn new(physical_volume: Box<str>) -> Self {
        Self { physical_volume, password: None, file_system: None }
    }

    pub fn password(mut self, pass: SecStr) -> Self {
        self.password = Some(pass);
        self
    }

    pub fn file_system(mut self, variant: impl Into<PartitionVariant>) -> Self {
        self.file_system = Some(Box::new(variant.into()));
        self
    }
}

impl From<LuksBuilder> for PartitionVariant {
    fn from(builder: LuksBuilder) -> PartitionVariant {
        PartitionVariant::Luks {
            physical_volume: builder.physical_volume,
            password:        builder.password,
            file_system:     builder.file_system,
        }
    }
}

pub struct FileSystemBuilder {
    file_system: FileSystem,
    label:       Option<Box<str>>,
}

impl FileSystemBuilder {
    pub fn new(file_system: FileSystem) -> Self { Self { file_system, label: None } }

    pub fn label(mut self, label: Box<str>) -> Self {
        self.label = Some(label);
        self
    }
}

impl From<FileSystemBuilder> for PartitionVariant {
    fn from(builder: FileSystemBuilder) -> PartitionVariant {
        PartitionVariant::FileSystem {
            file_system: builder.file_system,
            label:       builder.label,
        }
    }
}
