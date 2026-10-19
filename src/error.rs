//! The one error type a refused request returns.

use core::fmt;

/// Why a request was refused.
///
/// Each variant has a stable name, [`Error::code`], that a caller can match
/// on or log. The refusals the wire format names in its "Generation errors"
/// table carry exactly the name the conformance corpus gives them; the
/// others are this crate's own. The [`Display`](fmt::Display) text is a
/// sentence for a person and is not part of the contract.
///
/// The enum is `#[non_exhaustive]`: later versions add variants (decoding
/// refusals among them), so a `match` on it needs a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// A compact time whose first character would be the deterministic
    /// marker: the low 40 bits of the time are at or above 31 x 2^35. The
    /// code is `compact_time_out_of_range`, a corpus name.
    CompactTimeOutOfRange,
    /// Fixed random bytes whose count is not the random width of the
    /// request. The code is `rand_width_mismatch`, a corpus name.
    RandWidthMismatch,
    /// An explicit random width above 10 bytes. The code is
    /// `rand_size_too_large`, a corpus name.
    RandSizeTooLarge,
    /// A time above 2^48 - 1 milliseconds, the latest a time field holds, or
    /// a system clock reading before the Unix epoch. The code is
    /// `time_out_of_range`.
    TimeOutOfRange,
    /// An empty delimiter; the delimiter is a non-empty string. The code is
    /// `empty_delimiter`.
    EmptyDelimiter,
    /// A string that is not a size's short or long spelling, from
    /// [`Size`](crate::Size)'s `FromStr`. The code is `unknown_size`.
    UnknownSize,
    /// A [`ByteSource`](crate::ByteSource) could not fill its buffer. A byte
    /// source returns this variant when it fails. The code is `byte_source`.
    ByteSource,
}

impl Error {
    /// The refusal's stable name, in `snake_case`.
    ///
    /// ```
    /// assert_eq!(uxid::Error::RandSizeTooLarge.code(), "rand_size_too_large");
    /// ```
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::CompactTimeOutOfRange => "compact_time_out_of_range",
            Self::RandWidthMismatch => "rand_width_mismatch",
            Self::RandSizeTooLarge => "rand_size_too_large",
            Self::TimeOutOfRange => "time_out_of_range",
            Self::EmptyDelimiter => "empty_delimiter",
            Self::UnknownSize => "unknown_size",
            Self::ByteSource => "byte_source",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sentence = match self {
            Self::CompactTimeOutOfRange => {
                "compact time cannot encode this time: its first character would be the deterministic marker; use standard time"
            }
            Self::RandWidthMismatch => {
                "the fixed random bytes are not the random width of this request"
            }
            Self::RandSizeTooLarge => "the random width is above 10 bytes",
            Self::TimeOutOfRange => {
                "the time is outside 0 to 2^48 - 1 milliseconds since the Unix epoch"
            }
            Self::EmptyDelimiter => "the delimiter is empty",
            Self::UnknownSize => {
                "not a size: expected xs, s, m, l, xl, xsmall, small, medium, large or xlarge"
            }
            Self::ByteSource => "the byte source could not supply random bytes",
        };
        write!(f, "{sentence} ({})", self.code())
    }
}

impl core::error::Error for Error {}
