#[macro_use]
extern crate err_derive;
#[macro_use]
extern crate serde_derive;

mod lv;
mod pv;
mod vg;

pub use self::{lv::*, pv::*, vg::*};

use dbus::stdintf::org_freedesktop_dbus::{Introspectable, Properties};

#[derive(Deserialize)]
struct Nodes {
    #[serde(rename = "node", default)]
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    name: String,
}

pub trait LvmConn<'a>: Sized {
    const DEST: &'static str;
    const OBJECT: &'static str;

    type Item: LvmPath<'a>;

    fn conn(&'a self) -> &'a dbus::Connection;

    fn connect(&'a self, node: u32) -> Self::Item {
        let path = format!("{}/{}", Self::OBJECT, node);
        Self::Item::from_path(self.conn().with_path(Self::DEST, path, 1000), node)
    }

    fn connect_with_path(&'a self, path: dbus::Path<'a>) -> Self::Item {
        let node = path
            .as_cstr()
            .to_str()
            .expect("path is not UTF-8")
            .parse::<u32>()
            .expect("path is not a valid node");
        Self::Item::from_path(self.conn().with_path(Self::DEST, path, 1000), node)
    }
}

pub trait LvmPath<'a>: Sized {
    const PATH: &'static str;

    fn conn<'b>(&'b self) -> &'b dbus::ConnPath<'a, &'a dbus::Connection>;

    fn from_path(path: dbus::ConnPath<'a, &'a dbus::Connection>, node: u32) -> Self;

    fn id(&self) -> u32;

    fn get<T: for<'b> dbus::arg::Get<'b>>(&self, method: &'static str) -> Result<T, Error> {
        self.conn()
            .get::<T>(Self::PATH, method)
            .map_err(|why| MethodError::new(method, "VG", self.id(), why))
            .map_err(Error::from)
    }

    fn name(&self) -> Result<String, Error> { self.get("Name") }

    fn uuid(&self) -> Result<String, Error> { self.get("Uuid") }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "failed to establish dbus connection")]
    Connection(#[error(cause)] dbus::Error),
    #[error(display = "lvmdbus1 returned an error")]
    Method(#[error(cause)] MethodError),
}

#[derive(Debug, Error)]
#[error(display = "failed to call {} from {} {}", method, variant, id)]
pub struct MethodError {
    method: &'static str,
    variant: &'static str,
    id: u32,
    #[error(cause)]
    cause: dbus::Error,
}

impl MethodError {
    pub fn new(method: &'static str, variant: &'static str, id: u32, cause: dbus::Error) -> Self {
        Self { method, variant, id, cause }
    }
}

impl From<MethodError> for Error {
    fn from(error: MethodError) -> Self { Error::Method(error) }
}
