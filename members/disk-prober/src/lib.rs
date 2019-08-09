#[macro_use]
extern crate err_derive;

mod block;
mod partitions;

pub use self::{block::*, partitions::*};

use disk_types::{LvmLv, LvmPv};
use std::{fs, io, path::PathBuf};

use lvmdbus1::{LvConn, LvmConn, LvmPath, PvConn, VgConn};

#[derive(Debug, Error)]
#[error(display = "LVM probe error")]
pub struct LvmProbeError {
    #[error(cause)]
    cause: lvmdbus1::Error,
}

impl From<lvmdbus1::Error> for LvmProbeError {
    fn from(cause: lvmdbus1::Error) -> Self { LvmProbeError { cause } }
}

pub struct LvmProber {
    volume_groups:    VgConn,
    physical_volumes: PvConn,
}

impl LvmProber {
    pub fn new() -> Result<Self, LvmProbeError> {
        Ok(Self {
            volume_groups:    VgConn::new().map_err(LvmProbeError::from)?,
            physical_volumes: PvConn::new().map_err(LvmProbeError::from)?,
        })
    }

    pub fn iter_pvs<'a>(
        &'a self,
    ) -> impl Iterator<Item = Result<(u32, LvmPv), LvmProbeError>> + 'a {
        self.physical_volumes.iter().map(|pv| {
            let path = PathBuf::from(pv.name()?).into();
            let uuid = pv.uuid()?.into();
            let size_bytes = pv.size_bytes()?;

            Ok((pv.node, LvmPv { path, uuid, size_bytes }))
        })
    }

    pub fn iter_vgs<'a>(&'a self) -> impl Iterator<Item = Result<VgInfo, LvmProbeError>> + 'a {
        self.volume_groups.iter().map(|vg| {
            Ok(VgInfo {
                name:         vg.name()?,
                extent_size:  vg.extent_size_bytes()?,
                extents:      vg.extent_count()?,
                extents_free: vg.extent_free_count()?,
                pvs:          vg
                    .pvs()
                    .map(|path| {
                        let conn = PvConn::new()?;
                        let pv = conn.connect_with_path(path);

                        let path = PathBuf::from(pv.name()?).into();
                        let uuid = pv.uuid()?.into();
                        let size_bytes = pv.size_bytes()?;

                        Ok((pv.node, LvmPv { path, uuid, size_bytes }))
                    })
                    .collect::<Result<_, lvmdbus1::Error>>()?,
                lvs:          vg
                    .lvs()
                    .map(|path| {
                        let conn = LvConn::new()?;
                        let lv = conn.connect_with_path(path);

                        Ok(LvmLv {
                            name: lv.name()?.into(),
                            uuid: lv.uuid()?.into(),
                            path: lv.path()?.into(),
                        })
                    })
                    .collect::<Result<_, lvmdbus1::Error>>()?,
            })
        })
    }
}

pub struct VgInfo {
    pub name:         String,
    pub extent_size:  u64,
    pub extents:      u64,
    pub extents_free: u64,
    pub pvs:          Vec<(u32, LvmPv)>,
    pub lvs:          Vec<LvmLv>,
}

pub fn slaves_iter(device: &str) -> impl Iterator<Item = Box<str>> {
    let dir = PathBuf::from(["/sys/class/block/", device, "/slaves"].concat());

    fs::read_dir(dir).ok().into_iter().flat_map(|readdir| {
        readdir
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .map(Box::from)
    })
}
