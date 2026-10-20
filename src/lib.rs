//! Prefixed, K-sortable identifiers a person can read, wire-compatible with
//! the Elixir [uxid](https://github.com/riddler/uxid-ex) package.
//!
//! An id is an optional prefix, a delimiter and a body: a 48-bit millisecond
//! time (or a compact 40-bit one) followed by a random field, in Crockford
//! Base32, as uxid-ex's wire-format guide lays it out.
//!
//! ```
//! use uxid::{Options, Size};
//!
//! # #[cfg(all(feature = "std", feature = "getrandom"))] {
//! let id = uxid::generate(&Options::new().prefix("usr"))?;
//! assert!(id.starts_with("usr_"));
//!
//! let short = uxid::generate(&Options::new().prefix("cus").size(Size::S))?;
//! assert_eq!(short.len(), "cus_".len() + 14);
//! # }
//! # Ok::<(), uxid::Error>(())
//! ```
//!
//! # Features
//!
//! - `std` (default): `now_millis`, the system clock in milliseconds since
//!   the Unix epoch.
//! - `getrandom` (default): `GetrandomSource`, the platform's secure byte
//!   source.
//! - `std` and `getrandom` together: `generate`, which is
//!   [`generate_with`] with `GetrandomSource` and `now_millis`.
//! - `serde`: declared for later use; it derives nothing yet.
//!
//! With no feature the crate is `no_std` with `alloc`: [`generate_with`],
//! [`Options`], [`ByteSource`] and [`Error`] are all present, and the
//! caller supplies the byte source and the time.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod base32;
#[cfg(feature = "std")]
mod clock;
mod error;
mod generate;
mod options;
mod source;

#[cfg(feature = "std")]
pub use clock::now_millis;
pub use error::Error;
#[cfg(all(feature = "std", feature = "getrandom"))]
pub use generate::generate;
pub use generate::generate_with;
pub use options::{Case, Options, Size};
pub use source::ByteSource;
#[cfg(feature = "getrandom")]
pub use source::GetrandomSource;

#[cfg(test)]
mod tests {
    // Sabotage: with `extern crate alloc;` deleted, this test no longer compiles.
    #[test]
    fn links_alloc_without_std() {
        let prefix = alloc::string::String::from("usr");
        assert_eq!(prefix.len(), 3);
    }
}
