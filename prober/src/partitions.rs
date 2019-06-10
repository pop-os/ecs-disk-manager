use std::{
    convert::TryFrom,
    fs, io,
    iter::Skip,
    path::{Path, PathBuf},
    str::{FromStr, Lines},
};

const PARTITIONS_FILE: &str = "/proc/partitions";

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum EntryError {
    NanMajor,
    NanMinor,
    NanBlocks,
    NoMajor,
    NoMinor,
    NoBlocks,
    NoName,
}

pub struct PartitionEntry<'a> {
    pub major:  u16,
    pub minor:  u16,
    pub blocks: u64,
    pub name:   &'a str,
}

impl<'a> TryFrom<&'a str> for PartitionEntry<'a> {
    type Error = EntryError;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let mut fields = input.split_whitespace();

        fn try_parse<'a, T: FromStr>(
            iter: &mut dyn Iterator<Item = &'a str>,
            error1: EntryError,
            error2: EntryError,
        ) -> Result<T, EntryError> {
            iter.next().ok_or(error1).and_then(|string| T::from_str(string).map_err(|_| error2))
        }

        use EntryError::*;

        Ok(Self {
            major:  try_parse::<u16>(&mut fields, NoMajor, NanMajor)?,
            minor:  try_parse::<u16>(&mut fields, NoMinor, NanMinor)?,
            blocks: try_parse::<u64>(&mut fields, NoBlocks, NanBlocks)?,
            name:   fields.next().ok_or(NoName)?,
        })
    }
}

pub struct PartitionsFile(String);

impl PartitionsFile {
    pub fn new() -> io::Result<Self> { Self::from_file(Path::new(PARTITIONS_FILE)) }

    pub fn from_file(file: &Path) -> io::Result<Self> {
        fs::read_to_string(file).map(PartitionsFile)
    }
}

impl From<String> for PartitionsFile {
    fn from(input: String) -> Self { Self(input) }
}

impl From<&str> for PartitionsFile {
    fn from(input: &str) -> Self { Self(input.into()) }
}

pub struct EntryIter<'a>(Skip<Lines<'a>>);

impl<'a> IntoIterator for &'a PartitionsFile {
    type IntoIter = EntryIter<'a>;
    type Item = Result<PartitionEntry<'a>, EntryError>;

    fn into_iter(self) -> Self::IntoIter { EntryIter(self.0.lines().skip(2)) }
}

impl<'a> Iterator for EntryIter<'a> {
    type Item = Result<PartitionEntry<'a>, EntryError>;

    fn next(&mut self) -> Option<Self::Item> { self.0.next().map(PartitionEntry::try_from) }
}

pub struct DeviceIter<'a>(EntryIter<'a>);

impl<'a> From<EntryIter<'a>> for DeviceIter<'a> {
    fn from(iter: EntryIter<'a>) -> Self { Self(iter) }
}

impl<'a> Iterator for DeviceIter<'a> {
    type Item = Result<(PartitionEntry<'a>, Box<Path>), EntryError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|res| {
            res.map(|entry| {
                let path = PathBuf::from(["/dev/", entry.name].concat()).into();
                (entry, path)
            })
        })
    }
}
