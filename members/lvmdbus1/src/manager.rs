use crate::Error;
use dbus::{BusType, Connection, Message};

pub struct Manager {
    conn: Connection,
}

const DEST: &str = "com.redhat.lvmdbus1";
const INTERFACE: &str = "com.redhat.lvmdbus1.Manager";
const PATH: &str = "com/redhat/lvmdbus1/Manager";

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "argument mismatch in {} method", _0)]
    ArgumentMismatch(&'static str, #[error(cause)] dbus::arg::TypeMismatchError),
    #[error(display = "calling {} method failed", _0)]
    Call(&'static str, #[error(cause)] dbus::Error),
    #[error(display = "unable to establish dbus connection")]
    Connection(#[error(cause)] dbus::Error),
    #[error(display = "failed to create {} method call", _0)]
    NewMethodCall(&'static str, String),
}

impl Manager {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { conn: Connection::get_private(BusType::System).map_err(Error::Connection)? })
    }

    fn call_method<F: FnOnce(Message) -> Message>(
        &self,
        method: &'static str,
        append_args: F,
    ) -> Result<Message, Error> {
        let mut m = Message::new_method_call(DEST, PATH, INTERFACE, method)
            .map_err(|why| Error::NewMethodCall(method, why))?;

        m = append_args(m);

        self.send_with_reply_and_block(m, TIMEOUT)
            .map_err(|why| Error::Call(method, why))
    }
}
