#[macro_use]
extern crate serde_derive;

#[derive(Deserialize)]
struct Nodes {
    #[serde(rename = "node", default)]
    nodes: Vec<Node>
}

#[derive(Deserialize)]
struct Node {
    name: String
}

use self::vg::*;
use self::lv::*;
use self::pv::*;

mod lv {
    use dbus::{self, arg, BusType, Connection, ConnPath};
    use dbus::stdintf::org_freedesktop_dbus::Properties;
    use dbus::stdintf::org_freedesktop_dbus::Introspectable;
    use crate::Nodes;

    pub struct LvConn {
        conn: Connection
    }

    impl LvConn {
        pub fn new() -> Result<Self, dbus::Error> {
            Ok(Self { conn: Connection::get_private(BusType::System)? })
        }

        pub fn iter<'a>(&'a self) -> impl Iterator<Item = LvPath<'a>> {
            let path = self.conn.with_path(
                "com.redhat.lvmdbus1",
                "/com/redhat/lvmdbus1/Lv",
                1000
            );

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

        pub fn connect(&self, id: &str) -> LvPath {
            let node = ["/com/redhat/lvmdbus1/Lv/", id].concat();
            LvPath {
                conn: self.conn.with_path("com.redhat.lvmdbus1", node, 1000)
            }
        }
    }

    pub struct LvPath<'a> {
        conn: ConnPath<'a, &'a Connection>
    }

    impl<'a> LvPath<'a> {

    }
}

mod pv {
    use dbus::{self, arg, BusType, Connection, ConnPath};
    use dbus::stdintf::org_freedesktop_dbus::Properties;
    use dbus::stdintf::org_freedesktop_dbus::Introspectable;
    use crate::Nodes;

    pub struct PvConn {
        conn: Connection
    }

    impl PvConn {
        pub fn new() -> Result<Self, dbus::Error> {
            Ok(Self { conn: Connection::get_private(BusType::System)? })
        }

        pub fn iter<'a>(&'a self) -> impl Iterator<Item = PvPath<'a>> {
            let path = self.conn.with_path(
                "com.redhat.lvmdbus1",
                "/com/redhat/lvmdbus1/Pv",
                1000
            );

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

        pub fn connect(&self, id: &str) -> PvPath {
            let node = ["/com/redhat/lvmdbus1/Pv/", id].concat();
            PvPath {
                conn: self.conn.with_path("com.redhat.lvmdbus1", node, 1000)
            }
        }
    }

    pub struct PvPath<'a> {
        conn: ConnPath<'a, &'a Connection>
    }

    impl<'a> PvPath<'a> {

    }
}

mod vg {
    use dbus::{self, arg, BusType, Connection, ConnPath};
    use dbus::stdintf::org_freedesktop_dbus::Properties;
    use dbus::stdintf::org_freedesktop_dbus::Introspectable;

    use crate::Nodes;
    use crate::pv::PvPath;

    pub struct VgConn {
        conn: Connection
    }

    impl VgConn {
        pub fn new() -> Result<Self, dbus::Error> {
            Ok(Self { conn: Connection::get_private(BusType::System)? })
        }

        pub fn iter<'a>(&'a self) -> impl Iterator<Item = VgPath<'a>> {
            let path = self.conn.with_path(
                "com.redhat.lvmdbus1",
                "/com/redhat/lvmdbus1/Vg",
                1000
            );

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

        pub fn connect(&self, id: &str) -> VgPath {
            let node = ["/com/redhat/lvmdbus1/Vg/", id].concat();
            VgPath {
                conn: self.conn.with_path("com.redhat.lvmdbus1", node, 1000)
            }
        }
    }

    pub struct VgPath<'a> {
        conn: ConnPath<'a, &'a Connection>
    }

    impl<'a> VgPath<'a> {
        pub fn name(&self) -> Result<String, dbus::Error> {
            self.conn.get::<String>("com.redhat.lvmdbus1.Vg", "Name")
        }

        pub fn uuid(&self) -> Result<String, dbus::Error> {
            self.conn.get("com.redhat.lvmdbus1.Vg", "Uuid")
        }

        pub fn extent_count(&self) -> Result<u64, dbus::Error> {
            self.conn.get("com.redhat.Lvmdbus1.Vg", "ExtentCount")
        }

        pub fn extent_size_bytes(&self) -> Result<u64, dbus::Error> {
            self.conn.get("com.redhat.Lvmdbus1.Vg", "ExtentSizeBytes")
        }

        pub fn extent_free_count(&self) -> Result<u64, dbus::Error> {
            self.conn.get("com.redhat.Lvmdbus1.Vg", "FreeCount")
        }

        pub fn pv_count(&self) -> Result<u64, dbus::Error> {
            self.conn.get("com.redhat.Lvmdbus1.Vg", "PvCount")
        }

        pub fn pvs<'a>(&'a self) -> impl Iterator<Item = PvPath<'a>> {
            self.conn.get("com.redhat.Lvmdbus1.Vg", "Pvs")
                .ok()
                .into_iter()
                .flat_map
        }
    }
}
