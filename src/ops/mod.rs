pub mod create;
pub mod info;
pub mod luks;
pub mod modify;

use std::{io, process::ExitStatus};

#[derive(Debug, Error)]
pub enum CommandError {
    #[error(display = "command exited with failure status: {}", 0)]
    ExitStatis(ExitStatus),
    #[error(display = "failed to write to the stdin of the child process")]
    StdinWrite(#[error(cause)] io::Error),
    #[error(display = "failed to spawn child process")]
    Spawn(#[error(cause)] io::Error),
    #[error(display = "failed to wait on child process")]
    Wait(#[error(cause)] io::Error),
}
