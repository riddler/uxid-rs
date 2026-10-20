//! The one error type a refused request or a refused decode returns.

use core::fmt;

/// Why a request or a decode was refused.
///
/// Each variant has a stable name, [`Error::code`], that a caller can match
/// on or log. The refusals the wire format names, in its "Refusals" table
/// (decoding) and its "Generation errors" table, carry exactly the name the
/// conformance corpus gives them; the others are this crate's own. The
/// [`Display`](fmt::Display) text is a sentence for a person and is not part
/// of the contract.
///
/// The enum is `#[non_exhaustive]`: later versions add variants, so a
/// `match` on it needs a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Fixed random bytes asked for with a deterministic body
    /// ([`Options::from`](crate::Options::from)), which has no random field.
    /// The code is `rand_with_from`, a corpus name.
    RandWithFrom,
    /// A deterministic body ([`Options::from`](crate::Options::from)) asked
    /// for with a [`Monotonic`](crate::Monotonic) generator: a deterministic
    /// id has no random field to step. The code is `from_with_monotonic`, a
    /// corpus name.
    FromWithMonotonic,
    /// Fixed random bytes ([`Options::rand`](crate::Options::rand)) asked
    /// for with a [`Monotonic`](crate::Monotonic) generator, which chooses
    /// the random field itself. The code is `rand_with_monotonic`, a corpus
    /// name.
    RandWithMonotonic,
    /// A [`Monotonic`](crate::Monotonic) generator asked for with a random
    /// width of 0 bytes (a standard `xs`, or an explicit `rand_size` of 0),
    /// which leaves no random field to step. The code is
    /// `empty_rand_with_monotonic`, this crate's own: the wire format's
    /// generation errors name no such refusal.
    EmptyRandWithMonotonic,
    /// A time above 2^48 - 1 milliseconds, the latest a time field holds, a
    /// monotonic step that would carry the time past it, or a system clock
    /// reading before the Unix epoch. The code is
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
    /// A decoded body that is empty: an empty string, or one ending in the
    /// delimiter. The code is `empty`, a corpus name.
    Empty,
    /// A decoded body whose length in bytes is not one of the lengths the
    /// wire format's length table lists. The code is `invalid_length`, a
    /// corpus name.
    InvalidLength,
    /// A decoded body holding a character that is not a Crockford Base32
    /// symbol in either case. The code is `invalid_character`, a corpus name.
    InvalidCharacter,
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
            Self::RandWithFrom => "rand_with_from",
            Self::FromWithMonotonic => "from_with_monotonic",
            Self::RandWithMonotonic => "rand_with_monotonic",
            Self::EmptyRandWithMonotonic => "empty_rand_with_monotonic",
            Self::TimeOutOfRange => "time_out_of_range",
            Self::EmptyDelimiter => "empty_delimiter",
            Self::UnknownSize => "unknown_size",
            Self::ByteSource => "byte_source",
            Self::Empty => "empty",
            Self::InvalidLength => "invalid_length",
            Self::InvalidCharacter => "invalid_character",
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
            Self::RandWithFrom => {
                "fixed random bytes cannot be used with from: a deterministic id has no random field"
            }
            Self::FromWithMonotonic => {
                "from cannot be used with a monotonic generator: a deterministic id has no random field to step"
            }
            Self::RandWithMonotonic => {
                "fixed random bytes cannot be used with a monotonic generator: it chooses the random field"
            }
            Self::EmptyRandWithMonotonic => {
                "a monotonic generator needs a random field, but this random width is 0 bytes"
            }
            Self::TimeOutOfRange => {
                "the time is outside 0 to 2^48 - 1 milliseconds since the Unix epoch"
            }
            Self::EmptyDelimiter => "the delimiter is empty",
            Self::UnknownSize => {
                "not a size: expected xs, s, m, l, xl, xsmall, small, medium, large or xlarge"
            }
            Self::ByteSource => "the byte source could not supply random bytes",
            Self::Empty => "the id has an empty body",
            Self::InvalidLength => {
                "the id's body length in bytes is not 10, 13, 14, 18, 21, 22, 24 or 26"
            }
            Self::InvalidCharacter => {
                "the id's body holds a character outside the Crockford Base32 alphabet"
            }
        };
        write!(f, "{sentence} ({})", self.code())
    }
}

impl core::error::Error for Error {}
