//! Where random bytes come from: the [`ByteSource`] trait and, behind the
//! `getrandom` feature, [`GetrandomSource`], the platform's secure source.

use alloc::boxed::Box;

use crate::Error;

/// A source of random bytes for an id's random field.
///
/// [`generate_with`](crate::generate_with) calls [`fill`](ByteSource::fill)
/// at most once per id, with a buffer exactly the random width of the
/// request, and never when the random field is empty or fixed by
/// [`Options::rand`](crate::Options::rand), or when the id is deterministic
/// ([`Options::from`](crate::Options::from)).
///
/// A source for real ids must be cryptographically secure: the random field
/// is what makes an id hard to guess. `GetrandomSource`, behind the
/// `getrandom` feature, is the platform's.
/// A deterministic source belongs in tests only.
///
/// ```
/// use uxid::{ByteSource, Error, Options, Size, generate_with};
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
/// let options = Options::new().prefix("cus").size(Size::S);
/// let id = generate_with(&options, &mut Repeat(0xff), 1_700_000_000_000)?;
/// assert_eq!(id, "cus_01hf7yat00zzz1");
/// # Ok::<(), Error>(())
/// ```
pub trait ByteSource {
    /// Fills every byte of `buf` with random bytes.
    ///
    /// # Errors
    ///
    /// A source that cannot supply the bytes returns an error, usually
    /// [`Error::ByteSource`]; generation returns it unchanged.
    ///
    /// ```
    /// use uxid::{ByteSource, Error};
    ///
    /// fn draw(source: &mut impl ByteSource) -> Result<[u8; 4], Error> {
    ///     let mut buf = [0u8; 4];
    ///     source.fill(&mut buf)?;
    ///     Ok(buf)
    /// }
    ///
    /// # struct Repeat(u8);
    /// # impl ByteSource for Repeat {
    /// #     fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> { buf.fill(self.0); Ok(()) }
    /// # }
    /// assert_eq!(draw(&mut Repeat(7))?, [7, 7, 7, 7]);
    /// # Ok::<(), Error>(())
    /// ```
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error>;
}

/// A mutable reference to a source is a source: it fills from the source it
/// points at.
impl<T: ByteSource + ?Sized> ByteSource for &mut T {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        (**self).fill(buf)
    }
}

/// A boxed source is a source, so a `Box<dyn ByteSource>` can be chosen at run
/// time.
impl<T: ByteSource + ?Sized> ByteSource for Box<T> {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        (**self).fill(buf)
    }
}

/// The platform's cryptographically secure random source, through the
/// [`getrandom`](https://docs.rs/getrandom) crate; the source `generate`
/// uses. Behind the `getrandom` feature.
///
/// ```
/// use uxid::{ByteSource, GetrandomSource};
///
/// let mut buf = [0u8; 10];
/// GetrandomSource.fill(&mut buf)?;
/// # Ok::<(), uxid::Error>(())
/// ```
#[cfg(feature = "getrandom")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GetrandomSource;

#[cfg(feature = "getrandom")]
impl ByteSource for GetrandomSource {
    /// Fills `buf` from the platform source.
    ///
    /// # Errors
    ///
    /// [`Error::ByteSource`] when the platform source fails.
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        getrandom::fill(buf).map_err(|_| Error::ByteSource)
    }
}
