use disk_ops::table::{Gpt, PartitionError, Partitioner};
use disk_types::*;
use std::path::Path;

pub fn open_partitioner<E>(
    disk: &Disk,
    path: &Path,
    mut partitioner_func: impl FnMut(
        Result<&mut dyn Partitioner, PartitionError>,
        PartitionTable,
    ) -> Result<(), E>,
) -> Result<(), E> {
    let table = disk
        .table
        .expect("attempted to open a partition table for a device which did not have one");

    // Temporary variables for storing could-be table values.
    let mut gpt: Gpt;

    // Fetch a generic partitioner depending on the table kind.
    let partitioner: Result<&mut dyn Partitioner, PartitionError> = match table {
        PartitionTable::Guid => match Gpt::open(path) {
            Ok(guid) => {
                gpt = guid;
                Ok(&mut gpt)
            }
            Err(why) => Err(why),
        },
        PartitionTable::Mbr => unimplemented!("no mbr support"),
    };

    partitioner_func(partitioner, table)
}
