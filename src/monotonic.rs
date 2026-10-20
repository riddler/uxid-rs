//! Monotonic generation: [`Monotonic`], a generator whose ids increase
//! strictly, also within one millisecond and when the clock moves back.

use alloc::string::String;

use crate::base32;
use crate::generate::{MARKER, MAX_RAND_BYTES, MAX_TIME, push_prefix};
use crate::{ByteSource, Case, Error, Options, Size};

/// A monotonic generator: the ids it makes increase strictly, in byte order
/// as in time order, also when several fall in one millisecond and when the
/// clock moves back. It follows the "Monotonic generation" rule of uxid-ex's
/// wire format. A monotonic id is an ordinary time-based id of its size:
/// nothing in it says how it was made, and [`decode_with`](crate::decode_with)
/// reads it as any other.
///
/// The generator is built for one prefix and one random width, from the
/// options [`Monotonic::new`] takes, and keeps the last time `T` it gave and
/// the last random value `R` (the random field read as one big-endian
/// unsigned integer of `w` bytes). For a call at time `now`:
///
/// 1. with no last id, or `now` later than `T`, the id carries `now` and a
///    fresh random value, the seed: `w` bytes from the byte source, read
///    big-endian;
/// 2. with `now` equal to or earlier than `T`, the id carries `T` and
///    `R + step`, where `step` is drawn uniformly from 1 to 2^(4w)
///    inclusive (half the field's bit width): the low `4w` bits of
///    `ceil(w / 2)` bytes from the byte source, read big-endian, plus one.
///    The range holds a power of two values, so the masked bits are exactly
///    uniform over it, with no modulo bias and nothing redrawn;
/// 3. when `R + step` is 2^(8w) or more, the id carries `T + 1` and a fresh
///    seed instead.
///
/// A compact time holds only the low 40 bits of the time, so compact ids
/// keep that order only within one 40-bit window, as every compact id does.
///
/// The step is not a plus-one counter: consecutive ids in a millisecond are
/// spread over a window of 2^(4w) values. That stops trivial enumeration but
/// is a mitigation, not unpredictability, and a small random field stays
/// small: a standard `s` id has 16 random bits, a standard `m` id 40.
///
/// # One generator per thread, or your own lock
///
/// Every method that generates takes `&mut self`, and the state lives in the
/// value alone: there is no static, global or thread-local state, and two
/// generators never affect each other. Give each thread its own generator,
/// or share one behind a lock of your own, such as a `std::sync::Mutex`.
/// `Monotonic` is `Send` and `Sync`; `Sync` matters little, as a shared
/// reference cannot generate. Ids from two generators (one per thread, say)
/// carry no order between them within a millisecond. The type is not
/// `Clone`, so its state is never duplicated by accident.
///
/// ```
/// use uxid::{Error, Monotonic, Options, Size};
///
/// # #[cfg(feature = "getrandom")] {
/// let mut ids = Monotonic::new(&Options::new().prefix("usr").size(Size::M))?;
/// let now = 1_700_000_000_000;
/// let first = ids.generate_with(&mut uxid::GetrandomSource, now)?;
/// let second = ids.generate_with(&mut uxid::GetrandomSource, now)?;
/// let third = ids.generate_with(&mut uxid::GetrandomSource, now - 5)?;
/// assert!(first < second && second < third);
/// # }
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug)]
pub struct Monotonic {
    prefix: String,
    delimiter: String,
    case: Case,
    compact_time: bool,
    width: usize,
    time: Option<u64>,
    last: Option<Last>,
}

/// The last id's time and random value.
#[derive(Debug, Clone, Copy)]
struct Last {
    time: u64,
    value: u128,
}

impl Monotonic {
    /// Builds a generator from `options`: its prefix (an empty one is no
    /// prefix), size, random width, compact time, case and delimiter, each
    /// read as [`generate_with`](crate::generate_with) reads it, with the
    /// same defaults. A time set with [`Options::time`] is the time of every
    /// call, in place of `now`, as it is for `generate_with`; ids then still
    /// increase, stepping within that millisecond and moving past it on
    /// overflow.
    ///
    /// One default differs: at size `xs`, compact time is on unless
    /// [`compact_time`](Options::compact_time) was called, because a standard
    /// `xs` has no random field to step; a compact `xs` has a one-byte field.
    /// This holds with an explicit [`rand_size`](Options::rand_size) too.
    /// The random width is the size's, or the explicit `rand_size`, and must
    /// be at least one byte, so an `xs` with `compact_time(false)` and no
    /// `rand_size` is refused. An explicit `rand_size` on a standard `xs` is
    /// taken as [`generate_with`](crate::generate_with) takes it: the body has
    /// the same layout, but a length a decoder reads as another size.
    ///
    /// ```
    /// use uxid::{Error, Monotonic, Options, Size};
    ///
    /// let xs = Options::new().prefix("usr").size(Size::Xs);
    /// assert!(Monotonic::new(&xs).is_ok());
    /// let standard = xs.compact_time(false);
    /// assert_eq!(Monotonic::new(&standard).err(), Some(Error::EmptyRandWithMonotonic));
    /// ```
    ///
    /// # Errors
    ///
    /// - [`Error::EmptyDelimiter`] for an empty delimiter.
    /// - [`Error::FromWithMonotonic`] for [`Options::from`]: a deterministic
    ///   id has no random field to step.
    /// - [`Error::RandWithMonotonic`] for fixed [`Options::rand`] bytes: the
    ///   generator chooses the random field.
    /// - [`Error::RandSizeTooLarge`] for a random width above 10 bytes.
    /// - [`Error::EmptyRandWithMonotonic`] for a random width of 0 bytes: an
    ///   `xs` with `compact_time(false)`, or `rand_size(0)`.
    /// - [`Error::TimeOutOfRange`] for a time above 2^48 - 1.
    pub fn new(options: &Options<'_>) -> Result<Self, Error> {
        if options.delimiter.is_empty() {
            return Err(Error::EmptyDelimiter);
        }
        if options.from.is_some() {
            return Err(Error::FromWithMonotonic);
        }
        if options.rand.is_some() {
            return Err(Error::RandWithMonotonic);
        }
        let compact_time = options
            .compact_time
            .unwrap_or(matches!(options.size, Size::Xs));
        let width = match options.rand_size {
            Some(width) if width > MAX_RAND_BYTES => return Err(Error::RandSizeTooLarge),
            Some(width) => width,
            None => options.size.rand_bytes(compact_time),
        };
        if width == 0 {
            return Err(Error::EmptyRandWithMonotonic);
        }
        if options.time.is_some_and(|time| time > MAX_TIME) {
            return Err(Error::TimeOutOfRange);
        }
        Ok(Self {
            prefix: String::from(options.prefix),
            delimiter: String::from(options.delimiter),
            case: options.case,
            compact_time,
            width,
            time: options.time,
            last: None,
        })
    }

    /// Generates the next id, taking random bytes from `source` and the time
    /// from `now` (milliseconds since the Unix epoch) unless the options set
    /// one. The id carries `now`, the last id's time, or the millisecond
    /// after it, as the rule above says. A call that returns an error leaves
    /// the generator as it was.
    ///
    /// This form needs no feature and builds under `no_std`. `generate`,
    /// behind the `std` and `getrandom` features, is this method with the
    /// platform source and the system clock.
    ///
    /// ```
    /// use uxid::{ByteSource, Error, Monotonic, Options, Size};
    ///
    /// /// Writes the same byte every time: for a test, never for real ids.
    /// struct Repeat(u8);
    ///
    /// impl ByteSource for Repeat {
    ///     fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
    ///         buf.fill(self.0);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut ids = Monotonic::new(&Options::new().prefix("usr").size(Size::S))?;
    /// let now = 1_700_000_000_000;
    /// // The first id seeds the random field; the next, in the same
    /// // millisecond, steps it.
    /// assert_eq!(ids.generate_with(&mut Repeat(0), now)?, "usr_01hf7yat000000");
    /// assert_eq!(ids.generate_with(&mut Repeat(0), now)?, "usr_01hf7yat000001");
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// - [`Error::TimeOutOfRange`] for a `now` above 2^48 - 1, or a step that
    ///   overflows the field in millisecond 2^48 - 1, the last a time field
    ///   holds.
    /// - [`Error::CompactTimeOutOfRange`] for a compact time the id would
    ///   carry whose first character would be the deterministic marker. When
    ///   that time is the millisecond after the last id's, reached by an
    ///   overflow, the refusal can repeat while `now` stays at or before the
    ///   last id's time.
    /// - Whatever error `source` returns, unchanged.
    pub fn generate_with<S>(&mut self, source: &mut S, now: u64) -> Result<String, Error>
    where
        S: ByteSource + ?Sized,
    {
        let now = self.time.unwrap_or(now);
        if now > MAX_TIME {
            return Err(Error::TimeOutOfRange);
        }
        let next = match self.last {
            Some(last) if now <= last.time => {
                let value = last.value + self.step(source)?;
                if value < 1 << (8 * self.width) {
                    Last {
                        time: last.time,
                        value,
                    }
                } else if last.time == MAX_TIME {
                    return Err(Error::TimeOutOfRange);
                } else {
                    Last {
                        time: last.time + 1,
                        value: self.seed(source)?,
                    }
                }
            }
            _ => Last {
                time: now,
                value: self.seed(source)?,
            },
        };
        let compact_low = next.time % (1 << 40);
        if self.compact_time && compact_low >> 35 == MARKER {
            return Err(Error::CompactTimeOutOfRange);
        }

        let mut id = String::with_capacity(self.prefix.len() + self.delimiter.len() + 26);
        push_prefix(&mut id, &self.prefix, &self.delimiter);
        if self.compact_time {
            base32::push_integer(&mut id, compact_low, 8, self.case);
        } else {
            base32::push_integer(&mut id, next.time, 10, self.case);
        }
        let bytes = next.value.to_be_bytes();
        base32::push_bytes(&mut id, &bytes[bytes.len() - self.width..], self.case);
        self.last = Some(next);
        Ok(id)
    }

    /// Generates the next id, with random bytes from the platform's secure
    /// source ([`GetrandomSource`](crate::GetrandomSource)) and the time from
    /// [`now_millis`](crate::now_millis) unless the options set one. Behind
    /// the `std` and `getrandom` features, both on by default.
    ///
    /// ```
    /// use uxid::{Monotonic, Options};
    ///
    /// let mut ids = Monotonic::new(&Options::new().prefix("usr"))?;
    /// let first = ids.generate()?;
    /// let second = ids.generate()?;
    /// assert!(first < second);
    /// # Ok::<(), uxid::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`generate_with`](Self::generate_with), with [`Error::ByteSource`]
    /// when the platform source fails and [`Error::TimeOutOfRange`] when the
    /// system clock reads before the Unix epoch.
    #[cfg(all(feature = "std", feature = "getrandom"))]
    pub fn generate(&mut self) -> Result<String, Error> {
        let now = if self.time.is_some() {
            0
        } else {
            crate::now_millis()?
        };
        self.generate_with(&mut crate::GetrandomSource, now)
    }

    /// A fresh random value: `width` bytes from `source`, read big-endian.
    fn seed<S: ByteSource + ?Sized>(&self, source: &mut S) -> Result<u128, Error> {
        draw(source, self.width)
    }

    /// A step from 1 to 2^(4w): the low `4w` bits of `ceil(w / 2)` bytes from
    /// `source`, read big-endian, plus one.
    fn step<S: ByteSource + ?Sized>(&self, source: &mut S) -> Result<u128, Error> {
        let bits = 4 * self.width;
        let drawn = draw(source, self.width.div_ceil(2))?;
        Ok((drawn & ((1 << bits) - 1)) + 1)
    }
}

/// `count` bytes from `source`, read as one big-endian unsigned integer.
fn draw<S: ByteSource + ?Sized>(source: &mut S, count: usize) -> Result<u128, Error> {
    let mut buf = [0u8; MAX_RAND_BYTES];
    let buf = &mut buf[..count];
    source.fill(buf)?;
    Ok(buf
        .iter()
        .fold(0, |value, &byte| (value << 8) | u128::from(byte)))
}
