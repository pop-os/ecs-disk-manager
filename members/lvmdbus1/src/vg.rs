use dbus::{
    self,
    stdintf::org_freedesktop_dbus::{Introspectable, Properties},
    BusType, ConnPath, Connection,
};

use crate::{Error, LvmConn, LvmPath, Nodes};

pub struct VgConn {
    conn: Connection,
}

impl VgConn {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = VgPath<'a>> {
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

impl<'a> LvmConn<'a> for VgConn {
    type Item = VgPath<'a>;

    const DEST: &'static str = "com.redhat.lvmdbus1";
    const OBJECT: &'static str = "/com/redhat/lvmdbus1/Vg";

    fn conn(&self) -> &Connection { &self.conn }
}

pub struct VgPath<'a> {
    conn: ConnPath<'a, &'a Connection>,
    node: u32,
}

impl<'a> VgPath<'a> {
    pub fn extent_count(&self) -> Result<u64, Error> { self.get("ExtentCount") }

    pub fn extent_size_bytes(&self) -> Result<u64, Error> { self.get("ExtentSizeBytes") }

    pub fn extent_free_count(&self) -> Result<u64, Error> { self.get("FreeCount") }

    pub fn lv_count(&self) -> Result<u64, Error> { self.get("LvCount") }

    pub fn lvs(&self) -> impl Iterator<Item = dbus::Path> {
        self.conn
            .get::<Vec<dbus::Path>>(Self::PATH, "Lvs")
            .ok()
            .into_iter()
            .flat_map(|paths| paths.into_iter())
    }

    pub fn pv_count(&self) -> Result<u64, Error> { self.get("PvCount") }

    pub fn pvs(&self) -> impl Iterator<Item = dbus::Path> {
        self.conn
            .get::<Vec<dbus::Path>>(Self::PATH, "Pvs")
            .ok()
            .into_iter()
            .flat_map(|paths| paths.into_iter())
    }
}

impl<'a> LvmPath<'a> for VgPath<'a> {
    const PATH: &'static str = "com.redhat.lvmdbus1.Vg";

    fn conn<'b>(&'b self) -> &'b ConnPath<'a, &'a Connection> { &self.conn }

    fn id(&self) -> u32 { self.node }

    fn from_path(conn: ConnPath<'a, &'a Connection>, node: u32) -> Self { Self { conn, node } }
}
