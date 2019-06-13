use std::{io, path::Path};

pub fn wipe_signatures(device: &Path) -> io::Result<()> { unimplemented!() }

pub mod table {
    use disk_types::PartitionTable;
    use std::{io, path::Path};

    pub fn create(device: &Path, table: PartitionTable) -> io::Result<()> { unimplemented!() }

    pub fn delete(device: &Path, table: PartitionTable, at: u64) -> io::Result<()> {
        unimplemented!()
    }

    pub fn move_(device: &Path, table: PartitionTable, at: u64, offset: i64) -> io::Result<()> {
        unimplemented!()
    }

    pub fn resize(device: &Path, table: PartitionTable, at: u64, to: u64) -> io::Result<()> {
        unimplemented!()
    }
}

pub mod partition {
    use disk_types::FileSystem;
    use std::{io, path::Path};

    pub fn create(device: &Path, fs: FileSystem) -> io::Result<()> { unimplemented!() }

    pub fn resize(device: &Path, fs: FileSystem) -> io::Result<()> { unimplemented!() }
}
