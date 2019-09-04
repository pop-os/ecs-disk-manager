use crate::Error;
use dbus::{BusType, Connection, Message};

pub struct Manager {
    conn: Connection,
}

const DEST: &str = "com.redhat.lvmdbus1";
const INTERFACE: &str = "com.redhat.lvmdbus1.Manager";
const PATH: &str = "com/redhat/lvmdbus1/Manager";

impl Manager {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    // fn method_call<F: FnOnce(Message) -> Message>(&self, method: &'static str, append_args: F) {
    //     let mut m = Message::new_method_call(DEST, PATH, INTERFACE, method)
    // }
}
