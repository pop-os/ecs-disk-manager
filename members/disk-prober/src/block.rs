use super::*;
use blkid::*;
use disk_types::PartitionTable;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum BlockProbeError {
    Io(io::Error),
    Utf8Device,
    BlkId(BlkIdError),
    Entry(EntryError),
    GetPartition(u32),
    LoopWithoutBackingFile(io::Error),
    MapWithoutName(io::Error),
    PartitionNo(BlkIdError),
    PartitionNew(BlkIdError),
    PartitionProbe(BlkIdError),
    Probe(BlkIdError),
    Sectors(BlkIdError),
    Size(BlkIdError),
    Topology(BlkIdError),
    UnknownTable(Box<str>),
}

pub struct BlockProber(PartitionsFile);

impl BlockProber {
    pub fn new() -> Result<Self, BlockProbeError> {
        PartitionsFile::new().map(Self).map_err(BlockProbeError::Io)
    }
}

impl<'a> IntoIterator for &'a BlockProber {
    type IntoIter = BlockProberIter<'a>;
    type Item = Result<Option<Probed<'a>>, BlockProbeError>;

    fn into_iter(self) -> Self::IntoIter {
        BlockProberIter::from(DeviceIter::from(self.0.into_iter()))
    }
}

pub struct BlockProberIter<'a>(DeviceIter<'a>);

impl<'a> From<DeviceIter<'a>> for BlockProberIter<'a> {
    fn from(iter: DeviceIter<'a>) -> Self { Self(iter) }
}

impl<'a> Iterator for BlockProberIter<'a> {
    type Item = Result<Option<Probed<'a>>, BlockProbeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|result| {
            result.map_err(BlockProbeError::Entry).and_then(|(entry, path)| {
                let probe = Probe::new_from(&path).map_err(BlockProbeError::BlkId)?;
                if probe.is_wholedisk().unwrap_or(false) {
                    Ok(Some(Probed { entry, path, probe }))
                } else {
                    Ok(None)
                }
            })
        })
    }
}

pub struct Probed<'a> {
    pub entry: PartitionEntry<'a>,
    pub path:  Box<Path>,
    pub probe: Probe,
}

impl<'a> Probed<'a> {
    pub fn probe<'b>(&'b self) -> Result<ProbeInfo<'a, 'b>, BlockProbeError> {
        self.probe.probe_full().map_err(BlockProbeError::Probe)?;

        let size = self.probe.get_size().map_err(BlockProbeError::Size)?;
        let sectors = self.probe.get_sectors().map_err(BlockProbeError::Sectors)?;
        let physical_sector_size;
        let logical_sector_size;
        let alignment;

        {
            let topology = self.probe.get_topology().map_err(BlockProbeError::Topology)?;
            alignment = topology.get_alignment_offset();
            physical_sector_size = topology.get_physical_sector_size();
            logical_sector_size = topology.get_logical_sector_size();
        }

        let fstype = self.probe.lookup_value("TYPE").ok().map(Box::from);
        let uuid = self.probe.lookup_value("UUID").ok().map(Box::from);
        let mut table = None;
        let mut partitions = Vec::new();

        if let Ok(list) = self.probe.get_partitions() {
            table = list
                .get_table()
                .map(Table::get_type)
                .map(|string| {
                    string
                        .parse::<PartitionTable>()
                        .map_err(|_| BlockProbeError::UnknownTable(string.into()))
                })
                .transpose()?;

            if let Ok(nparts) = list.numof_partitions() {
                for porder in 0..nparts {
                    let partition = list
                        .get_partition(porder)
                        .ok_or_else(|| BlockProbeError::GetPartition(porder))?;

                    let partno = partition.get_partno().map_err(BlockProbeError::PartitionNo)?;

                    let nvme = self.entry.name.chars().last().map_or(false, char::is_numeric);
                    let modifier = if nvme { "p" } else { "" };
                    let device = format!("{}{}{}", self.entry.name, modifier, partno);
                    let path = PathBuf::from(format!("/dev/{}", device));

                    let probe = Probe::new_from(&path).map_err(BlockProbeError::PartitionNew)?;

                    probe.probe_full().map_err(BlockProbeError::PartitionProbe)?;
                    partitions.push(ProbePartInfo {
                        device:    Box::from(device),
                        no:        partno,
                        path:      Box::from(path),
                        sectors:   partition.get_size(),
                        offset:    partition.get_start(),
                        partlabel: partition.get_name().map(Box::from),
                        partuuid:  partition.get_uuid().map(Box::from),
                        uuid:      probe.lookup_value("UUID").ok().map(Box::from),
                        fstype:    probe.lookup_value("TYPE").ok().map(Box::from),
                    });
                }
            }
        }

        let variant = if self.entry.name.starts_with("dm-") {
            let dm_name_path = ["/sys/class/block/", self.entry.name, "/dm/name"].concat();
            let mut dm_name = fs::read_to_string(dm_name_path.as_str())
                .map_err(BlockProbeError::MapWithoutName)?;
            dm_name.pop();

            DeviceVariant::Map(dm_name.into())
        } else if self.entry.name.starts_with("loop") {
            let backing_path =
                ["/sys/class/block/", self.entry.name, "/loop/backing_file"].concat();
            let mut backing = fs::read_to_string(backing_path.as_str())
                .map_err(BlockProbeError::LoopWithoutBackingFile)?;
            backing.pop();
            DeviceVariant::Loopback(PathBuf::from(backing).into())
        } else {
            DeviceVariant::Physical(table)
        };

        Ok(ProbeInfo {
            alignment,
            device: self.entry.name,
            devno_major: self.entry.major,
            devno_minor: self.entry.minor,
            path: &self.path,
            variant,
            size,
            sectors,
            logical_sector_size,
            physical_sector_size,
            fstype,
            uuid,
            partitions,
        })
    }
}

pub struct ProbeInfo<'a, 'b> {
    pub alignment:            u64,
    pub device:               &'a str,
    pub devno_major:          u16,
    pub devno_minor:          u16,
    pub logical_sector_size:  u64,
    pub partitions:           Vec<ProbePartInfo>,
    pub path:                 &'b Path,
    pub physical_sector_size: u64,
    pub sectors:              u64,
    pub size:                 u64,
    pub fstype:               Option<Box<str>>,
    pub uuid:                 Option<Box<str>>,
    pub variant:              DeviceVariant,
}

pub struct ProbePartInfo {
    pub device:    Box<str>,
    pub no:        u32,
    pub offset:    u64,
    pub partlabel: Option<Box<str>>,
    pub partuuid:  Option<Box<str>>,
    pub path:      Box<Path>,
    pub sectors:   u64,
    pub fstype:    Option<Box<str>>,
    pub uuid:      Option<Box<str>>,
}

#[derive(Debug)]
pub enum DeviceVariant {
    Loopback(Box<Path>),
    Map(Box<str>),
    Physical(Option<PartitionTable>),
}
