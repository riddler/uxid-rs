//! Time-based id generation: a time field followed by a random field, under
//! an optional prefix.

use alloc::string::String;

use crate::base32;
use crate::{ByteSource, Error, Options};

/// The latest time a time field holds: 2^48 - 1 milliseconds.
const MAX_TIME: u64 = (1 << 48) - 1;
/// The widest random field at any size, in bytes.
const MAX_RAND_BYTES: usize = 10;
/// The deterministic marker's value, which a compact time may not start with.
const MARKER: u64 = 31;

/// Generates a time-based id from `options`, taking random bytes from
/// `source` and the time from `now` (milliseconds since the Unix epoch) when
/// the options set none.
///
/// This form needs no feature: it reads no clock and no platform source, so
/// it builds under `no_std`. [`generate`](crate::generate) is this function
/// with the platform source and the system clock.
///
/// ```
/// use uxid::{Error, Options, Size, generate_with};
///
/// # #[cfg(feature = "getrandom")] {
/// let id = generate_with(&Options::new().prefix("usr"), &mut uxid::GetrandomSource, 1_700_000_000_000)?;
/// assert!(id.starts_with("usr_01hf7yat00"));
/// # }
///
/// // Fixed random bytes reproduce a test vector, with no source called.
/// # struct NoSource;
/// # impl uxid::ByteSource for NoSource {
/// #     fn fill(&mut self, _: &mut [u8]) -> Result<(), Error> { unreachable!() }
/// # }
/// let options = Options::new()
///     .prefix("cus")
///     .size(Size::S)
///     .time(1_700_000_000_000)
///     .rand(&[0x01, 0x23]);
/// assert_eq!(generate_with(&options, &mut NoSource, 0)?, "cus_01hf7yat0004h1");
/// # Ok::<(), Error>(())
/// ```
///
/// # Errors
///
/// - [`Error::EmptyDelimiter`] for an empty delimiter.
/// - [`Error::TimeOutOfRange`] for a time (or `now`) above 2^48 - 1.
/// - [`Error::CompactTimeOutOfRange`] for a compact time whose first
///   character would be the deterministic marker.
/// - [`Error::RandSizeTooLarge`] for a random width above 10 bytes.
/// - [`Error::RandWidthMismatch`] for fixed random bytes of another width.
/// - Whatever error `source` returns, unchanged.
pub fn generate_with<S>(options: &Options<'_>, source: &mut S, now: u64) -> Result<String, Error>
where
    S: ByteSource + ?Sized,
{
    let delimiter = options.delimiter;
    if delimiter.is_empty() {
        return Err(Error::EmptyDelimiter);
    }
    let time = options.time.unwrap_or(now);
    if time > MAX_TIME {
        return Err(Error::TimeOutOfRange);
    }
    let compact_low = time % (1 << 40);
    if options.compact_time && compact_low >> 35 == MARKER {
        return Err(Error::CompactTimeOutOfRange);
    }
    let width = match options.rand_size {
        Some(width) if width > MAX_RAND_BYTES => return Err(Error::RandSizeTooLarge),
        Some(width) => width,
        None => options.size.rand_bytes(options.compact_time),
    };

    let mut drawn = [0u8; MAX_RAND_BYTES];
    let rand = match options.rand {
        Some(fixed) if fixed.len() != width => return Err(Error::RandWidthMismatch),
        Some(fixed) => fixed,
        None => {
            let buf = &mut drawn[..width];
            if width > 0 {
                source.fill(buf)?;
            }
            &*buf
        }
    };

    let mut id = String::with_capacity(options.prefix.len() + delimiter.len() + 26);
    if !options.prefix.is_empty() {
        id.push_str(options.prefix);
        id.push_str(delimiter);
    }
    if options.compact_time {
        base32::push_integer(&mut id, compact_low, 8, options.case);
    } else {
        base32::push_integer(&mut id, time, 10, options.case);
    }
    base32::push_bytes(&mut id, rand, options.case);
    Ok(id)
}

/// Generates a time-based id from `options`, with random bytes from the
/// platform's secure source ([`GetrandomSource`](crate::GetrandomSource))
/// and the time from the system clock when the options set none. Behind the
/// `std` and `getrandom` features, both on by default.
///
/// ```
/// use uxid::{Options, Size};
///
/// let id = uxid::generate(&Options::new().prefix("usr").size(Size::M))?;
/// assert_eq!(id.len(), "usr_".len() + 18);
/// # Ok::<(), uxid::Error>(())
/// ```
///
/// # Errors
///
/// As [`generate_with`], with [`Error::ByteSource`] when the platform source
/// fails and [`Error::TimeOutOfRange`] when the system clock reads before
/// the Unix epoch.
#[cfg(all(feature = "std", feature = "getrandom"))]
pub fn generate(options: &Options<'_>) -> Result<String, Error> {
    let now = match options.time {
        Some(_) => 0,
        None => system_now()?,
    };
    generate_with(options, &mut crate::GetrandomSource, now)
}

/// The system clock in milliseconds since the Unix epoch.
#[cfg(all(feature = "std", feature = "getrandom"))]
fn system_now() -> Result<u64, Error> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| Error::TimeOutOfRange)?;
    u64::try_from(elapsed.as_millis()).map_err(|_| Error::TimeOutOfRange)
}
