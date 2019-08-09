use dbus::{self, stdintf::org_freedesktop_dbus::Introspectable, BusType, ConnPath, Connection};

use crate::{Error, LvmConn, LvmPath, Nodes};

pub struct PvConn {
    conn: Connection,
}

impl PvConn {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = PvPath<'a>> {
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

impl<'a> LvmConn<'a> for PvConn {
    type Item = PvPath<'a>;

    const DEST: &'static str = "com.redhat.lvmdbus1";
    const OBJECT: &'static str = "/com/redhat/lvmdbus1/Vg";

    fn conn(&self) -> &Connection { &self.conn }
}

pub struct PvPath<'a> {
    conn:     ConnPath<'a, &'a Connection>,
    pub node: u32,
}

impl<'a> LvmPath<'a> for PvPath<'a> {
    const PATH: &'static str = "com.redhat.lvmdbus1.Pv";

    fn conn<'b>(&'b self) -> &'b ConnPath<'a, &'a Connection> { &self.conn }

    fn id(&self) -> u32 { self.node }

    fn from_path(conn: ConnPath<'a, &'a Connection>, node: u32) -> Self { Self { conn, node } }
}

impl<'a> PvPath<'a> {
    pub fn size_bytes(&self) -> Result<u64, Error> { self.get("SizeBytes") }
}
