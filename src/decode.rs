//! Reading an id back: [`Decoded`], the decode functions and the validity
//! checks. Decoding never panics on malformed input; it answers a `Result`.

use alloc::string::String;

use crate::{Error, Size};

/// The delimiter `decode`, [`is_valid`] and [`is_deterministic`] split on.
const DEFAULT_DELIMITER: &str = "_";
/// The width of the compact time's window, 2^40 milliseconds.
const COMPACT_WINDOW: u64 = 1 << 40;
/// How far ahead of the decoder's clock a compact time may land before it is
/// read as one from the window before: one day in milliseconds.
const ONE_DAY: u64 = 86_400_000;

/// What a well-formed id says about itself: its prefix, the size and time
/// form its body length gives, whether its body is deterministic, and the
/// time it carries. The random field is not read back.
///
/// The fields are private and read through the accessors, so later versions
/// can add to the value without breaking a caller.
///
/// ```
/// use uxid::Size;
///
/// let decoded = uxid::decode_with("cus_01hf7yat0004h1", "_", 0)?;
/// assert_eq!(decoded.prefix(), Some("cus"));
/// assert_eq!(decoded.size(), Size::S);
/// assert!(!decoded.is_compact());
/// assert!(!decoded.is_deterministic());
/// assert_eq!(decoded.time(), Some(1_700_000_000_000));
/// # Ok::<(), uxid::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decoded {
    prefix: Option<String>,
    size: Size,
    compact: bool,
    deterministic: bool,
    time: Option<u64>,
}

impl Decoded {
    /// Everything before the delimiter [`decode_with`] splits at, or `None`
    /// when the id has no delimiter. An id that starts with the delimiter, which no producer
    /// emits, has the prefix `Some("")`, best read as no prefix.
    #[must_use]
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// The size the body length gives.
    #[must_use]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Whether the body length is a compact-time one (13, 21 or 24). The
    /// lengths 10 and 18 are always read as standard time, so a compact xs or
    /// compact m id decodes with this `false`.
    #[must_use]
    pub const fn is_compact(&self) -> bool {
        self.compact
    }

    /// Whether the body starts with the deterministic marker `z` or `Z`.
    #[must_use]
    pub const fn is_deterministic(&self) -> bool {
        self.deterministic
    }

    /// The time in milliseconds since the Unix epoch, or `None` for a
    /// deterministic body, which carries none. A compact time's high bits
    /// come from the decoder's clock.
    #[must_use]
    pub const fn time(&self) -> Option<u64> {
        self.time
    }
}

/// Decodes an id under the delimiter `delimiter`, reading a compact time
/// against the clock `now` (milliseconds since the Unix epoch).
///
/// This form needs no feature: it reads no clock, so it builds under
/// `no_std`. `decode`, behind the `std` feature, is this function with the
/// delimiter `_` and `now_millis`.
///
/// The body is everything after the last delimiter found reading left to
/// right, and the prefix everything before it; a string with no delimiter is
/// a body with no prefix. Matches found that way do not overlap, as in the
/// reference's split: under `--`, `usr---01hf7yat00` splits after `usr`,
/// not at the last `--`. The delimiter must be the one the id was generated
/// with, since an id does not record it. The body length in bytes gives the
/// size and the time form:
///
/// | Body length | Size | Time |
/// |------|------|------|
/// | 10 | `Xs` | standard |
/// | 13 | `S`  | compact |
/// | 14 | `S`  | standard |
/// | 18 | `M`  | standard |
/// | 21 | `L`  | compact |
/// | 22 | `L`  | standard |
/// | 24 | `Xl` | compact |
/// | 26 | `Xl` | standard |
///
/// The lengths 10 and 18 are also those of a compact xs and a compact m
/// body, and they are always read as standard time: such an id decodes to
/// its size, not compact, with a time read from its first 10 characters,
/// which is not the time it was generated at.
///
/// A compact body carries only the low 40 bits of its time; the high bits
/// are `now`'s, less one 2^40 ms window when the result would be more than
/// one day ahead of `now`. So the time comes back exact while the id and
/// `now` fall in the same 2^40 ms window (about 34.8 years) and the id is
/// no more than one day ahead of `now`. When `now` is in the first window
/// after the Unix epoch there is no window before it, and the time is the
/// one in `now`'s window: this crate's answer, which the conformance corpus
/// does not fix. `now` is read only for a compact body.
///
/// ```
/// use uxid::{Error, Size, decode_with};
///
/// let decoded = decode_with("usr_hf7yat0004hm5", "_", 1_700_000_000_000)?;
/// assert_eq!(decoded.size(), Size::S);
/// assert!(decoded.is_compact());
/// assert_eq!(decoded.time(), Some(1_700_000_000_000));
///
/// assert_eq!(decode_with("usr_", "_", 0), Err(Error::Empty));
/// assert_eq!(decode_with("usr_01hf7yat000", "_", 0), Err(Error::InvalidLength));
/// assert_eq!(decode_with("usr_01hu7yat00", "_", 0), Err(Error::InvalidCharacter));
/// # Ok::<(), Error>(())
/// ```
///
/// # Errors
///
/// - [`Error::EmptyDelimiter`] for an empty delimiter.
/// - [`Error::Empty`] for an empty body: an empty string, or one ending in
///   the delimiter.
/// - [`Error::InvalidLength`] for a body whose length in bytes is not in the
///   table above.
/// - [`Error::InvalidCharacter`] for a body character that is not a
///   Crockford Base32 symbol in either case.
///
/// A body that breaks more than one rule is refused for the first it breaks,
/// in the order above. Any input gets an answer: decoding never panics.
pub fn decode_with(id: &str, delimiter: &str, now: u64) -> Result<Decoded, Error> {
    decode_by(id, delimiter, || Ok(now))
}

/// Decodes an id under the delimiter `_`, reading a compact time against the
/// system clock ([`now_millis`](crate::now_millis)), which is read only for a
/// compact body. Behind the `std` feature, on by default; see
/// [`decode_with`] for the reading.
///
/// ```
/// let decoded = uxid::decode("usr_zcvt7epac0t1ebcsjfyf7cwz25")?;
/// assert_eq!(decoded.prefix(), Some("usr"));
/// assert!(decoded.is_deterministic());
/// assert_eq!(decoded.time(), None);
/// # Ok::<(), uxid::Error>(())
/// ```
///
/// # Errors
///
/// As [`decode_with`], with [`Error::TimeOutOfRange`] when a compact body
/// needs the system clock and it reads before the Unix epoch.
#[cfg(feature = "std")]
pub fn decode(id: &str) -> Result<Decoded, Error> {
    decode_by(id, DEFAULT_DELIMITER, crate::now_millis)
}

/// Whether `id` is a well-formed id under the delimiter `_`: whether
/// [`decode_with`] answers a value for it. Validity does not depend on the
/// clock, so this reads none and needs no feature.
///
/// For another delimiter, or to require a prefix, decode instead; the clock
/// passed does not change the answer:
///
/// ```
/// assert!(uxid::is_valid("cus_01hf7yat0004h1"));
/// assert!(!uxid::is_valid("cus_01hf7yat0004h12"));
/// assert!(!uxid::is_valid(""));
///
/// let id = "cus--01hf7yat0004h1";
/// assert!(uxid::decode_with(id, "--", 0).is_ok_and(|d| d.prefix() == Some("cus")));
/// ```
#[must_use]
pub fn is_valid(id: &str) -> bool {
    decode_with(id, DEFAULT_DELIMITER, 0).is_ok()
}

/// Whether `id` is a well-formed deterministic id under the delimiter `_`:
/// one [`decode_with`] answers a value for whose body starts with the marker
/// `z` or `Z`. A malformed string is not one, whatever its first body
/// character. It reads no clock and needs no feature.
///
/// ```
/// assert!(uxid::is_deterministic("usr_zcvt7epac0t1ebcsjfyf7cwz25"));
/// assert!(!uxid::is_deterministic("usr_01hf7yat0004h1"));
/// assert!(!uxid::is_deterministic("usr_zcvt7epac"));
/// ```
#[must_use]
pub fn is_deterministic(id: &str) -> bool {
    decode_with(id, DEFAULT_DELIMITER, 0).is_ok_and(|decoded| decoded.deterministic)
}

/// Decodes `id`, calling `clock` only for a compact body.
fn decode_by<C>(id: &str, delimiter: &str, clock: C) -> Result<Decoded, Error>
where
    C: FnOnce() -> Result<u64, Error>,
{
    if delimiter.is_empty() {
        return Err(Error::EmptyDelimiter);
    }
    let (prefix, body) = split(id, delimiter);
    let body = body.as_bytes();
    let (size, compact) = refuse(body)?;
    let deterministic = matches!(body.first(), Some(b'z' | b'Z'));
    let time = if deterministic {
        None
    } else if compact {
        Some(compact_time(read(&body[..8]), clock()?))
    } else {
        Some(standard_time(body))
    };
    Ok(Decoded {
        prefix: prefix.map(String::from),
        size,
        compact,
        deterministic,
        time,
    })
}

/// Splits `id` at the last delimiter found by a left-to-right scan whose
/// matches do not overlap, as the reference's split does; with no delimiter
/// the whole id is the body and there is no prefix.
fn split<'a>(id: &'a str, delimiter: &str) -> (Option<&'a str>, &'a str) {
    match id.match_indices(delimiter).last() {
        Some((at, _)) => (Some(&id[..at]), &id[at + delimiter.len()..]),
        None => (None, id),
    }
}

/// The size and time form of a well-formed body, or the first rule it
/// breaks: empty, then length in bytes, then alphabet.
fn refuse(body: &[u8]) -> Result<(Size, bool), Error> {
    if body.is_empty() {
        return Err(Error::Empty);
    }
    let form = match body.len() {
        10 => (Size::Xs, false),
        13 => (Size::S, true),
        14 => (Size::S, false),
        18 => (Size::M, false),
        21 => (Size::L, true),
        22 => (Size::L, false),
        24 => (Size::Xl, true),
        26 => (Size::Xl, false),
        _ => return Err(Error::InvalidLength),
    };
    if body.iter().all(|&byte| value(byte).is_some()) {
        Ok(form)
    } else {
        Err(Error::InvalidCharacter)
    }
}

/// The 5-bit value of a Crockford Base32 symbol in either case, or `None`
/// for any other byte.
const fn value(byte: u8) -> Option<u8> {
    let lower = byte.to_ascii_lowercase();
    match lower {
        b'0'..=b'9' => Some(lower - b'0'),
        b'a'..=b'h' => Some(lower - b'a' + 10),
        b'j' | b'k' => Some(lower - b'j' + 18),
        b'm' | b'n' => Some(lower - b'm' + 20),
        b'p'..=b't' => Some(lower - b'p' + 22),
        b'v'..=b'z' => Some(lower - b'v' + 27),
        _ => None,
    }
}

/// Reads symbols as one integer, 5 bits each, most significant first. The
/// caller passes symbols only.
fn read(symbols: &[u8]) -> u64 {
    symbols.iter().fold(0, |acc, &byte| {
        (acc << 5) | u64::from(value(byte).unwrap_or(0))
    })
}

/// The standard 48-bit time: the first symbol's low 3 bits, then nine 5-bit
/// groups. A producer never writes a first symbol above 7.
fn standard_time(body: &[u8]) -> u64 {
    (read(&body[..1]) & 7) << 45 | read(&body[1..10])
}

/// The compact time: the low 40 bits from the id and the high bits from the
/// clock, less one window when that lands more than a day ahead of the
/// clock and a window before it exists.
const fn compact_time(low: u64, now: u64) -> u64 {
    let time = (now >> 40) << 40 | low;
    if time > now && time - now > ONE_DAY && time >= COMPACT_WINDOW {
        time - COMPACT_WINDOW
    } else {
        time
    }
}

#[cfg(test)]
mod tests {
    use super::{compact_time, decode_by, value};
    use crate::Error;

    // Sabotage: calling the clock before the compact branch turns this red.
    #[test]
    fn the_clock_is_called_only_for_a_compact_body() {
        let failing = || Err(Error::TimeOutOfRange);
        assert!(decode_by("usr_01hf7yat0004h1", "_", failing).is_ok());
        assert!(decode_by("usr_zcvt7epac0t1e", "_", failing).is_ok());
        assert!(decode_by("usr_", "_", failing) == Err(Error::Empty));
        assert!(decode_by("usr_hf7yat0004hm5", "_", failing) == Err(Error::TimeOutOfRange));
    }

    // Sabotage: mapping `j` to 19 (as if `i` were a symbol) turns this red.
    #[test]
    fn every_symbol_reads_its_value_in_both_cases_and_nothing_else_does() {
        let lower = b"0123456789abcdefghjkmnpqrstvwxyz";
        let upper = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        for byte in 0..=u8::MAX {
            let expected = lower
                .iter()
                .position(|&b| b == byte)
                .or_else(|| upper.iter().position(|&b| b == byte))
                .map(|at| at as u8);
            assert_eq!(value(byte), expected, "byte {byte:#04x}");
        }
    }

    // Sabotage: dropping the `time >= COMPACT_WINDOW` condition, the subtraction wrapping, turns this red.
    #[test]
    fn a_compact_time_in_the_first_window_is_never_moved_below_zero() {
        let low = (1u64 << 40) - 1;
        assert_eq!(compact_time(low, 0), low);
        assert_eq!(compact_time(5, 0), 5);
    }

    // Sabotage: subtracting the window at exactly one day ahead (`>=`) turns this red.
    #[test]
    fn a_compact_time_exactly_one_day_ahead_stays_in_the_clock_window() {
        let now = 3 << 40;
        assert_eq!(compact_time(86_400_000, now), now + 86_400_000);
        assert_eq!(compact_time(86_400_001, now), now + 86_400_001 - (1 << 40));
    }
}
