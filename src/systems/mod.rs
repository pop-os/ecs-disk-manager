use crate::DiskManager;
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

pub mod remove;
mod scan;

pub use self::scan::scan;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "operations cancelled")]
    Cancelled,
    #[error(display = "failure in remove system")]
    Remove(#[error(cause)] remove::Error)
}

impl From<remove::Error> for Error {
    fn from(error: remove::Error) -> Self {
        Error::Remove(error)
    }
}


macro_rules! cancellation_check {
    ($cancel:ident) => {
        if $cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
    };
}

pub fn run(world: &mut DiskManager, cancel: &Arc<AtomicBool>) -> Result<(), Error> {
    // TODO: On error, unset any remove flags on entities.
    remove::run(world, cancel)?;

    cancellation_check!(cancel);

    Ok(())
}


