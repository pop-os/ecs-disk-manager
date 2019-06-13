use crate::{Error, LvmConn, LvmPath, MethodError, Nodes};
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
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = LvPath<'a>> {
        let path = self.conn().with_path("com.redhat.lvmdbus1", Self::OBJECT, 1000);

        path.introspect()
            .map_err(|why| {
                eprintln!("{:?}", why);
                why
            })
            .ok()
            .into_iter()
            .map(|xml| serde_xml_rs::from_str::<Nodes>(xml.as_str()).unwrap())
            .flat_map(|nodes| nodes.nodes)
            .filter_map(|node| node.name.parse::<u32>().ok())
            .map(move |id| self.connect(id))
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
    node: u32,
}

impl<'a> LvPath<'a> {
    pub fn path(&self) -> Result<PathBuf, Error> { self.get::<String>("Path").map(PathBuf::from) }

    pub fn size_bytes(&self) -> Result<u64, Error> { self.get("SizeBytes") }

    pub fn vg(&self) -> Result<dbus::Path, Error> { self.get("Vg") }
}

impl<'a> LvmPath<'a> for LvPath<'a> {
    const PATH: &'static str = "com.redhat.lvmdbus1.Lv";

    fn conn<'b>(&'b self) -> &'b ConnPath<'a, &'a Connection> { &self.conn }

    fn id(&self) -> u32 { self.node }

    fn from_path(conn: ConnPath<'a, &'a Connection>, node: u32) -> Self { Self { conn, node } }
}
