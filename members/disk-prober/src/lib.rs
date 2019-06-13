mod block;
mod partitions;

pub use self::{block::*, partitions::*};

use disk_types::{LvmLv, LvmPv};
use std::{fs, io, path::PathBuf};

use lvmdbus1::{LvConn, LvmConn, LvmPath, PvConn, VgConn};

#[derive(Debug)]
pub enum VgProbeError {
    Dbus(dbus::Error),
}

pub struct VgProber(VgConn);

impl VgProber {
    pub fn new() -> Result<Self, VgProbeError> {
        VgConn::new().map_err(VgProbeError::Dbus).map(Self)
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Result<VgInfo, VgProbeError>> + 'a {
        self.0.iter().map(|vg| {
            Ok(VgInfo {
                name:         vg.name().map_err(VgProbeError::Dbus)?,
                extent_size:  vg.extent_size_bytes().map_err(VgProbeError::Dbus)?,
                extents:      vg.extent_count().map_err(VgProbeError::Dbus)?,
                extents_free: vg.extent_free_count().map_err(VgProbeError::Dbus)?,
                pvs:          vg
                    .pvs()
                    .map(|path| {
                        let conn = PvConn::new().map_err(VgProbeError::Dbus)?;
                        let pv = conn.connect_with_path(path);

                        Ok(LvmPv {
                            path: PathBuf::from(pv.name().map_err(VgProbeError::Dbus)?).into(),
                            uuid: pv.uuid().map_err(VgProbeError::Dbus)?.into(),
                        })
                    })
                    .collect::<Result<_, VgProbeError>>()?,
                lvs:          vg
                    .lvs()
                    .map(|path| {
                        let conn = LvConn::new().map_err(VgProbeError::Dbus)?;
                        let lv = conn.connect_with_path(path);

                        Ok(LvmLv {
                            name: lv.name().map_err(VgProbeError::Dbus)?.into(),
                            uuid: lv.uuid().map_err(VgProbeError::Dbus)?.into(),
                            path: lv.path().map_err(VgProbeError::Dbus)?.into(),
                        })
                    })
                    .collect::<Result<_, VgProbeError>>()?,
            })
        })
    }
}

pub struct VgInfo {
    pub name:         String,
    pub extent_size:  u64,
    pub extents:      u64,
    pub extents_free: u64,
    pub pvs:          Vec<LvmPv>,
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
