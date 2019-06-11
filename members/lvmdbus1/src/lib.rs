#[macro_use]
extern crate serde_derive;

mod lv;
mod pv;
mod vg;

pub use self::vg::*;
pub use self::lv::*;
pub use self::pv::*;

use dbus::stdintf::org_freedesktop_dbus::Properties;

#[derive(Deserialize)]
struct Nodes {
    #[serde(rename = "node", default)]
    nodes: Vec<Node>
}

#[derive(Deserialize)]
struct Node {
    name: String
}

pub trait LvmConn<'a>: Sized {
    const DEST: &'static str;
    const OBJECT: &'static str;

    type Item: LvmPath<'a>;

    fn conn(&'a self) -> &'a dbus::Connection;

    fn connect(&'a self, node: &str) -> Self::Item {
        let node = [Self::OBJECT, "/", node].concat();
        Self::Item::from_path(self.conn().with_path(Self::DEST, node, 1000))
    }

    fn connect_with_path(&'a self, path: dbus::Path<'a>) -> Self::Item {
        Self::Item::from_path(self.conn().with_path(Self::DEST, path, 1000))
    }
}

pub trait LvmPath<'a>: Sized {
    const PATH: &'static str;

    fn conn<'b>(&'b self) -> &'b dbus::ConnPath<'a, &'a dbus::Connection>;

    fn from_path(path: dbus::ConnPath<'a, &'a dbus::Connection>) -> Self;

    fn name(&self) -> Result<String, dbus::Error> {
        self.conn().get(Self::PATH, "Name")
    }

    fn uuid(&self) -> Result<String, dbus::Error> {
        self.conn().get(Self::PATH, "Uuid")
    }
}
