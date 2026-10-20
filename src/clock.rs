//! The system clock, behind the `std` feature.

use crate::Error;

/// The system clock, in milliseconds since the Unix epoch: the time
/// `generate` uses when the options set none. Behind the `std` feature.
///
/// Pass it as `now` to [`generate_with`](crate::generate_with) to pair the
/// system clock with a byte source of your own.
///
/// ```
/// use uxid::{ByteSource, Error, Options, generate_with, now_millis};
///
/// # struct MySource;
/// # impl ByteSource for MySource {
/// #     fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> { buf.fill(7); Ok(()) }
/// # }
/// let id = generate_with(&Options::new().prefix("usr"), &mut MySource, now_millis()?)?;
/// assert!(id.starts_with("usr_"));
/// # Ok::<(), Error>(())
/// ```
///
/// # Errors
///
/// [`Error::TimeOutOfRange`] when the clock reads before the Unix epoch, or
/// so far after it that the milliseconds do not fit a `u64`.
pub fn now_millis() -> Result<u64, Error> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::TimeOutOfRange)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| Error::TimeOutOfRange)
}
