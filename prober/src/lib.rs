mod partitions;

pub use self::partitions::*;

use blkid::*;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum DiskProberError {
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
}

pub struct Prober(PartitionsFile);

impl Prober {
    pub fn new() -> Result<Self, DiskProberError> {
        PartitionsFile::new().map(Prober).map_err(DiskProberError::Io)
    }
}

impl<'a> IntoIterator for &'a Prober {
    type IntoIter = ProberIter<'a>;
    type Item = Result<Option<Probed<'a>>, DiskProberError>;

    fn into_iter(self) -> Self::IntoIter { ProberIter::from(DeviceIter::from(self.0.into_iter())) }
}

pub struct ProberIter<'a>(DeviceIter<'a>);

impl<'a> From<DeviceIter<'a>> for ProberIter<'a> {
    fn from(iter: DeviceIter<'a>) -> Self { Self(iter) }
}

impl<'a> Iterator for ProberIter<'a> {
    type Item = Result<Option<Probed<'a>>, DiskProberError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|result| {
            result.map_err(DiskProberError::Entry).and_then(|(entry, path)| {
                let probe = Probe::new_from(&path).map_err(DiskProberError::BlkId)?;
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
    pub path:  PathBuf,
    pub probe: Probe,
}

impl<'a> Probed<'a> {
    pub fn probe<'b>(&'b self) -> Result<ProbeInfo<'a, 'b>, DiskProberError> {
        self.probe.probe_full().map_err(DiskProberError::Probe)?;

        let size = self.probe.get_size().map_err(DiskProberError::Size)?;
        let sectors = self.probe.get_sectors().map_err(DiskProberError::Sectors)?;
        let physical_sector_size;
        let logical_sector_size;
        let alignment;

        {
            let topology = self.probe.get_topology().map_err(DiskProberError::Topology)?;
            alignment = topology.get_alignment_offset();
            physical_sector_size = topology.get_physical_sector_size();
            logical_sector_size = topology.get_logical_sector_size();
        }

        let type_ = self.probe.lookup_value("TYPE").ok().map(Box::from);
        let uuid = self.probe.lookup_value("UUID").ok().map(Box::from);
        let mut table = None;
        let mut partitions = Vec::new();

        if let Ok(list) = self.probe.get_partitions() {
            table = list.get_table().map(Table::get_type).map(Box::from);
            if let Ok(nparts) = list.numof_partitions() {
                for porder in 0..nparts {
                    let partition = list
                        .get_partition(porder)
                        .ok_or_else(|| DiskProberError::GetPartition(porder))?;

                    let partno = partition.get_partno().map_err(DiskProberError::PartitionNo)?;

                    let nvme = self.entry.name.chars().last().map_or(false, char::is_numeric);
                    let modifier = if nvme { "p" } else { "" };
                    let device = format!("{}{}{}", self.entry.name, modifier, partno);
                    let path = PathBuf::from(format!("/dev/{}", device));

                    let probe = Probe::new_from(&path).map_err(DiskProberError::PartitionNew)?;

                    probe.probe_full().map_err(DiskProberError::PartitionProbe)?;
                    partitions.push(ProbePartInfo {
                        device,
                        no: partno,
                        path,
                        sectors: partition.get_size(),
                        offset: partition.get_start(),
                        partlabel: partition.get_name().map(Box::from),
                        partuuid: partition.get_uuid().map(Box::from),
                        uuid: probe.lookup_value("UUID").ok().map(Box::from),
                        type_: probe.lookup_value("TYPE").ok().map(Box::from),
                    });
                }
            }
        }

        let variant = if self.entry.name.starts_with("dm-") {
            let dm_name_path = ["/sys/class/block/", self.entry.name, "/dm/name"].concat();
            let mut dm_name = fs::read_to_string(dm_name_path.as_str())
                .map_err(DiskProberError::MapWithoutName)?;
            dm_name.pop();

            DeviceVariant::Map(dm_name.into())
        } else if self.entry.name.starts_with("loop") {
            let backing_path =
                ["/sys/class/block/", self.entry.name, "/loop/backing_file"].concat();
            let mut backing = fs::read_to_string(backing_path.as_str())
                .map_err(DiskProberError::LoopWithoutBackingFile)?;
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
            type_,
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
    pub type_:                Option<Box<str>>,
    pub uuid:                 Option<Box<str>>,
    pub variant:              DeviceVariant,
}

pub struct ProbePartInfo {
    pub device:    String,
    pub no:        u32,
    pub offset:    u64,
    pub partlabel: Option<Box<str>>,
    pub partuuid:  Option<Box<str>>,
    pub path:      PathBuf,
    pub sectors:   u64,
    pub type_:     Option<Box<str>>,
    pub uuid:      Option<Box<str>>,
}

#[derive(Debug)]
pub enum DeviceVariant {
    Loopback(Box<Path>),
    Map(Box<str>),
    Physical(Option<Box<str>>),
}
