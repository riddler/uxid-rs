//! The options a time-based id is generated from: [`Options`], [`Size`] and
//! [`Case`].

use core::str::FromStr;

use crate::Error;

/// An id's size, which fixes the random field's width (and so the body
/// length) for standard and compact time.
///
/// | Size | Random bytes, standard | Random bytes, compact | Body length, standard | Body length, compact |
/// |------|------|------|------|------|
/// | `Xs` | 0  | 1  | 10 | 10 |
/// | `S`  | 2  | 3  | 14 | 13 |
/// | `M`  | 5  | 6  | 18 | 18 |
/// | `L`  | 7  | 8  | 22 | 21 |
/// | `Xl` | 10 | 10 | 26 | 24 |
///
/// It parses from the short spellings `xs`, `s`, `m`, `l` and `xl` and the
/// long ones `xsmall`, `small`, `medium`, `large` and `xlarge`:
///
/// ```
/// use uxid::Size;
///
/// assert_eq!("m".parse::<Size>(), Ok(Size::M));
/// assert_eq!("xlarge".parse::<Size>(), Ok(Size::Xl));
/// assert_eq!("huge".parse::<Size>(), Err(uxid::Error::UnknownSize));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Size {
    /// Extra small: no random field in standard time.
    Xs,
    /// Small.
    S,
    /// Medium.
    M,
    /// Large.
    L,
    /// Extra large, the default.
    #[default]
    Xl,
}

impl Size {
    /// The random field's width in bytes for this size and time form.
    pub(crate) const fn rand_bytes(self, compact_time: bool) -> usize {
        match (self, compact_time) {
            (Self::Xs, false) => 0,
            (Self::Xs, true) => 1,
            (Self::S, false) => 2,
            (Self::S, true) => 3,
            (Self::M, false) => 5,
            (Self::M, true) => 6,
            (Self::L, false) => 7,
            (Self::L, true) => 8,
            (Self::Xl, _) => 10,
        }
    }
}

impl FromStr for Size {
    type Err = Error;

    /// Parses a short or long spelling, in lower case exactly; anything else
    /// is [`Error::UnknownSize`].
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "xs" | "xsmall" => Ok(Self::Xs),
            "s" | "small" => Ok(Self::S),
            "m" | "medium" => Ok(Self::M),
            "l" | "large" => Ok(Self::L),
            "xl" | "xlarge" => Ok(Self::Xl),
            _ => Err(Error::UnknownSize),
        }
    }
}

/// The letter case of an id's body. Case never applies to the prefix, and an
/// upper-case body and its lower-case spelling are the same id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Case {
    /// Lower case, the default.
    #[default]
    Lower,
    /// Upper case.
    Upper,
}

/// The options of [`generate`](crate::generate) and
/// [`generate_with`](crate::generate_with).
///
/// Build one from [`Options::new`] (or [`Default`]) and the setters, each of
/// which takes and returns the value; an option left unset takes the default
/// its setter names. The struct is `#[non_exhaustive]`: later versions add
/// options.
///
/// ```
/// use uxid::{Case, Options, Size};
///
/// let options = Options::new()
///     .prefix("usr")
///     .size(Size::S)
///     .case(Case::Upper);
/// # let _ = options;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Options<'a> {
    pub(crate) prefix: &'a str,
    pub(crate) size: Size,
    pub(crate) rand_size: Option<usize>,
    pub(crate) compact_time: bool,
    pub(crate) case: Case,
    pub(crate) delimiter: &'a str,
    pub(crate) time: Option<u64>,
    pub(crate) rand: Option<&'a [u8]>,
}

impl Default for Options<'_> {
    /// The same as [`Options::new`].
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Options<'a> {
    /// The default options: no prefix, size `xl`, the size's random width,
    /// standard time, lower case, the delimiter `_`, the current time and
    /// random bytes from the byte source.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prefix: "",
            size: Size::Xl,
            rand_size: None,
            compact_time: false,
            case: Case::Lower,
            delimiter: "_",
            time: None,
            rand: None,
        }
    }

    /// The resource prefix, written before the delimiter exactly as given
    /// (case never applies to it). It may contain the delimiter. An empty
    /// string is no prefix, the default: the id is the bare body.
    #[must_use]
    pub const fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = prefix;
        self
    }

    /// The size, which fixes the random field's width. Defaults to
    /// [`Size::Xl`].
    #[must_use]
    pub const fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// An explicit random field width in bytes, from 0 to 10, which overrides
    /// the size's width; above 10 is [`Error::RandSizeTooLarge`]. A body of a
    /// width the size table does not list has the same layout, but its
    /// length is one a decoder refuses.
    #[must_use]
    pub const fn rand_size(mut self, bytes: usize) -> Self {
        self.rand_size = Some(bytes);
        self
    }

    /// Compact time: the low 40 bits of the time in 8 characters instead of
    /// 48 bits in 10, the freed byte handed to the random field (except at
    /// `xl`). A time whose first compact character would be the
    /// deterministic marker, from mid-2038 until the 40-bit window rolls over
    /// in 2039, is [`Error::CompactTimeOutOfRange`]. Defaults to `false`.
    #[must_use]
    pub const fn compact_time(mut self, compact_time: bool) -> Self {
        self.compact_time = compact_time;
        self
    }

    /// The letter case of the body. Defaults to [`Case::Lower`].
    #[must_use]
    pub const fn case(mut self, case: Case) -> Self {
        self.case = case;
        self
    }

    /// The string between the prefix and the body, written only when there
    /// is a prefix. Defaults to `_`; an empty delimiter is
    /// [`Error::EmptyDelimiter`].
    #[must_use]
    pub const fn delimiter(mut self, delimiter: &'a str) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// The time, in milliseconds since the Unix epoch, from 0 to 2^48 - 1;
    /// above that is [`Error::TimeOutOfRange`]. Defaults to the current
    /// time: the system clock for [`generate`](crate::generate), the `now`
    /// argument for [`generate_with`](crate::generate_with).
    #[must_use]
    pub const fn time(mut self, millis: u64) -> Self {
        self.time = Some(millis);
        self
    }

    /// Fixed random field bytes, exactly the random width of the request
    /// (any other count is [`Error::RandWidthMismatch`]); the byte source is
    /// then not called.
    ///
    /// This exists for reproducible ids, such as test vectors, and is never a
    /// source of randomness: an id built from fixed bytes is exactly as
    /// guessable as those bytes. Leave it unset for real ids.
    #[must_use]
    pub const fn rand(mut self, bytes: &'a [u8]) -> Self {
        self.rand = Some(bytes);
        self
    }
}
