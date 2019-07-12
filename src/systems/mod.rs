pub mod create;
pub mod format;
pub mod remove;
pub mod resize;
mod scan;

pub use self::scan::scan;

use crate::DiskManager;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "operations cancelled")]
    Cancelled,
    #[error(display = "failure in create system")]
    Create(#[error(cause)] create::Error),
    #[error(display = "failure in create system")]
    Format(#[error(cause)] format::Error),
    #[error(display = "failure in remove system")]
    Remove(#[error(cause)] remove::Error),
    // #[error(display = "failure in resize system")]
    // Resize(#[error(cause)] resize::Error),
}

impl From<create::Error> for Error {
    fn from(error: create::Error) -> Self {
        Error::Create(error)
    }
}

impl From<format::Error> for Error {
    fn from(error: format::Error) -> Self {
        Error::Format(error)
    }
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
    remove::run(world, cancel)?;

    // cancellation_check!(cancel);
    // resize::run(world, cancel)?;

    cancellation_check!(cancel);
    create::run(world, cancel)?;

    cancellation_check!(cancel);
    format::run(world, cancel)?;

    Ok(())
}
