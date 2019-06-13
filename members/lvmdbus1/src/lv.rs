use crate::{LvmConn, LvmPath, Nodes};
use dbus::{
    self, arg,
    stdintf::org_freedesktop_dbus::{Introspectable, Properties},
    BusType, ConnPath, Connection,
};
use std::path::PathBuf;

pub struct LvConn {
    conn: Connection,
}

impl LvConn {
    pub fn new() -> Result<Self, dbus::Error> {
        Ok(Self { conn: Connection::get_private(BusType::System)? })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = LvPath<'a>> {
        let path = self.conn.with_path("com.redhat.lvmdbus1", "/com/redhat/lvmdbus1/Lv", 1000);

        path.introspect()
            .map_err(|why| {
                eprintln!("{:?}", why);
                why
            })
            .ok()
            .into_iter()
            .map(|xml| serde_xml_rs::from_str::<Nodes>(xml.as_str()).unwrap())
            .flat_map(|nodes| nodes.nodes)
            .map(move |id| self.connect(&id.name))
    }
}

impl<'a> LvmConn<'a> for LvConn {
    type Item = LvPath<'a>;

    const DEST: &'static str = "com.redhat.lvmdbus1";
    const OBJECT: &'static str = "/com/redhat/lvmdbus1/Vg";

    fn conn(&self) -> &Connection { &self.conn }
}

pub struct LvPath<'a> {
    conn: ConnPath<'a, &'a Connection>,
}

impl<'a> LvPath<'a> {
    pub fn path(&self) -> Result<PathBuf, dbus::Error> {
        self.conn.get::<String>(Self::PATH, "Path").map(PathBuf::from)
    }

    pub fn size_bytes(&self) -> Result<u64, dbus::Error> { self.conn.get(Self::PATH, "SizeBytes") }

    pub fn vg(&self) -> Result<dbus::Path, dbus::Error> { self.conn.get(Self::PATH, "Vg") }
}

impl<'a> LvmPath<'a> for LvPath<'a> {
    const PATH: &'static str = "com.redhat.lvmdbus1.Lv";

    fn conn<'b>(&'b self) -> &'b ConnPath<'a, &'a Connection> { &self.conn }

    fn from_path(conn: ConnPath<'a, &'a Connection>) -> Self { Self { conn } }
}
