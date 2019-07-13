use disk_types::PartitionTable;
use std::{fs::{self, File, OpenOptions}, io::{self, Seek, SeekFrom}};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use gptman::{GPT, GPTPartitionEntry, PartitionName};
use rand::Rng;

use super::{Partitioner, PartitionError, PartitionResult, TableError};

pub fn convert_str_to_array(uuid: &str) -> Result<[u8; 16], ParseIntError> {
    let mut arr = [0; 16];
    let mut digits = uuid
        .chars()
        .filter(|&x| x != '-')
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|x| x.iter().collect::<String>())
        .map(|x| u8::from_str_radix(x.as_str(), 16))
        .collect::<Result<Vec<_>, ParseIntError>>()?;

    if digits.len() != 16 {
        panic!("must be 16 digits");
    }

    let mut reordered = Vec::new();
    reordered.extend(digits.drain(..4).rev());
    reordered.extend(digits.drain(..2).rev());
    reordered.extend(digits.drain(..2).rev());
    reordered.extend(digits.drain(..2));
    reordered.extend(digits.drain(..));

    for (e, v) in arr.iter_mut().zip(reordered.iter()) {
        *e = *v;
    }

    Ok(arr)
}

pub struct Gpt {
    device: File,
    table: GPT
}

impl Gpt {
    pub fn create(device: &Path, sector_size: u64) -> PartitionResult<Self> {
        let mut device = OpenOptions::new()
            .write(true)
            .open(device)
            .map_err(PartitionError::DeviceOpen)?;

        let table = GPT::new_from(&mut device, sector_size, generate_random_uuid())
            .map_err(TableError::from)
            .map_err(PartitionError::TableRead)?;

        write_protective_mbr_into(&mut device, sector_size).unwrap();

        Ok(Gpt { device, table })
    }

    pub fn open(device: &Path) -> PartitionResult<Self> {
        let mut device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(device)
            .map_err(PartitionError::DeviceOpen)?;

        let table = GPT::find_from(&mut device)
            .map_err(TableError::from)
            .map_err(PartitionError::TableRead)?;

        Ok(Gpt { device, table })
    }

    fn find(&self, sector: u64) -> PartitionResult<u32> {
        fn between(partition: &GPTPartitionEntry, sector: u64) -> bool {
            sector >= partition.starting_lba && sector <= partition.ending_lba
        }

        self.table.iter()
            .find(|(id, partition)| partition.is_used() && between(partition, sector))
            .map(|(id, _)| id)
            .ok_or(PartitionError::PartitionNotFound)
    }
}

impl Partitioner for Gpt {
    fn add(&mut self, start: u64, end: u64, name: Option<&str>) -> PartitionResult<u32> {
        let partition = GPTPartitionEntry {
            starting_lba: start,
            ending_lba: end,
            attribute_bits: 0,
            partition_name: name.unwrap_or("").into(),
            partition_type_guid: convert_str_to_array("0FC63DAF-8483-4772-8E79-3D69D8477DE4").unwrap(),
            unique_parition_guid: generate_random_uuid(),
        };

        let id = self.table.iter()
            .find(|(_, info)| !info.is_used())
            .map(|(id, _)| id)
            .ok_or(PartitionError::LimitExceeded)?;

        self.table[id] = partition;

        Ok(id)
    }

    fn label(&mut self, sector: u64, label: &str) -> PartitionResult<()> {
        let needle = self.find(sector)?;
        for (id, entry) in self.table.iter_mut() {
            if id == needle {
                entry.partition_name = PartitionName::from(label);
                return Ok(());
            }
        }

        Err(PartitionError::PartitionNotFound)
    }

    fn last_sector(&self) -> u64 {
        self.table.header.last_usable_lba
    }

    fn remove(&mut self, sector: u64) -> PartitionResult<()> {
        self.table.remove(self.find(sector)?)
            .map_err(TableError::from)
            .map_err(PartitionError::PartitionRemove)?;

        Ok(())
    }

    fn write(&mut self) -> PartitionResult<()> {
        eprintln!("writing table");
        self.table.write_into(&mut self.device)
            .map_err(TableError::from)
            .map_err(PartitionError::DeviceWrite)?;

        eprintln!("reloading table");
        gptman::linux::reread_partition_table(&mut self.device)
            .map_err(PartitionError::TableReload)?;

        Ok(())
    }
}

fn generate_random_uuid() -> [u8; 16] {
    rand::thread_rng().gen()
}

pub fn wipe(device: &Path) -> io::Result<()> {
    std::process::Command::new("wipefs")
        .arg("-a")
        .arg(device)
        .output()
        .map(|_| ())
}


use bincode::serialize_into;
use std::io::Write;

pub fn write_protective_mbr_into<W: ?Sized>(mut writer: &mut W, sector_size: u64) -> bincode::Result<()>
where
    W: Write + Seek,
{
    let size = writer.seek(SeekFrom::End(0))? / sector_size - 1;
    writer.seek(SeekFrom::Start(446))?;
    // partition 1
    writer.write_all(&[
        0x00, // status
        0x00, 0x02, 0x00, // CHS address of first absolute sector
        0xee, // partition type
        0xff, 0xff, 0xff, // CHS address of last absolute sector
        0x01, 0x00, 0x00, 0x00, // LBA of first absolute sector
    ])?;

    // number of sectors in partition 1
    serialize_into(
        &mut writer,
        &(if size > u64::from(u32::max_value()) {
            u32::max_value()
        } else {
            size as u32
        }),
    )?;

    writer.write_all(&[0; 16])?; // partition 2
    writer.write_all(&[0; 16])?; // partition 3
    writer.write_all(&[0; 16])?; // partition 4
    writer.write_all(&[0x55, 0xaa])?; // signature

    Ok(())
}
