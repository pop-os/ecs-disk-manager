use std::io;

/// Custom error handling for the library
#[derive(Debug, Error)]
pub enum BlkIdError {
    #[error(display = "I/O error: {}", _0)]
    Io(io::Error),
    #[error(display = "non-UTF8 string")]
    InvalidStr,
}

pub(crate) trait CResult: Copy {
    fn is_error(self) -> bool;
}

impl CResult for i32 {
    fn is_error(self) -> bool { self < 0 }
}

impl CResult for i64 {
    fn is_error(self) -> bool { self < 0 }
}

impl<T> CResult for *const T {
    fn is_error(self) -> bool { self.is_null() }
}

impl<T> CResult for *mut T {
    fn is_error(self) -> bool { self.is_null() }
}

pub(crate) fn cvt<T: CResult>(result: T) -> Result<T, BlkIdError> {
    if result.is_error() {
        return Err(BlkIdError::Io(io::Error::last_os_error()));
    }

    Ok(result)
}
