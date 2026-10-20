//! Prefixed, K-sortable identifiers a person can read, wire-compatible with
//! the Elixir [uxid](https://github.com/riddler/uxid-ex) package.
//!
//! An id is an optional prefix, a delimiter and a body: a 48-bit millisecond
//! time (or a compact 40-bit one) followed by a random field, in Crockford
//! Base32, as uxid-ex's wire-format guide lays it out. Given
//! [`Options::from`], the body is deterministic instead: the marker `z`
//! (`Z` in upper case) and characters of a SHA-256 hash of the prefix and that string, so the same
//! inputs always give the same id.
//!
//! A [`Monotonic`] generator, a value the caller holds, makes ids that
//! increase strictly, also within one millisecond.
//!
//! [`decode_with`] reads an id back to a [`Decoded`] (its prefix, size, time
//! form, whether it is deterministic, and its time), or refuses it with the
//! wire format's reason; it never panics on malformed input. [`is_valid`]
//! and [`is_deterministic`] answer the two common questions about a string.
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
//!
//! let named = uxid::generate(&Options::new().prefix("usr").from("alice@example.com"))?;
//! assert_eq!(named, "usr_zcvt7epac0t1ebcsjfyf7cwz25");
//! assert!(uxid::is_deterministic(&named));
//! # }
//!
//! let decoded = uxid::decode_with("cus_01hf7yat0004h1", "_", 0)?;
//! assert_eq!(decoded.time(), Some(1_700_000_000_000));
//! assert_eq!(uxid::decode_with("cus_", "_", 0), Err(uxid::Error::Empty));
//! # Ok::<(), uxid::Error>(())
//! ```
//!
//! # Features
//!
//! - `std` (default): `now_millis`, the system clock in milliseconds since
//!   the Unix epoch, and `decode`, which is [`decode_with`] with the
//!   delimiter `_` and that clock.
//! - `getrandom` (default): `GetrandomSource`, the platform's secure byte
//!   source.
//! - `std` and `getrandom` together: `generate`, which is
//!   [`generate_with`] with `GetrandomSource` and `now_millis`, and
//!   [`Monotonic`]'s `generate`, which is its `generate_with` with the same
//!   two.
//! - `serde`: declared for later use; it derives nothing yet.
//!
//! With no feature the crate is `no_std` with `alloc`: [`generate_with`],
//! [`decode_with`], [`is_valid`], [`is_deterministic`], [`Options`],
//! [`Decoded`], [`Monotonic`] with its `generate_with`, [`ByteSource`] and
//! [`Error`] are all present, and the caller
//! supplies the byte source and the time (a deterministic id reads neither,
//! and a decode reads the time only for a compact body).

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

mod base32;
#[cfg(feature = "std")]
mod clock;
mod decode;
mod deterministic;
mod error;
mod generate;
mod monotonic;
mod options;
mod source;

#[cfg(feature = "std")]
pub use clock::now_millis;
#[cfg(feature = "std")]
pub use decode::decode;
pub use decode::{Decoded, decode_with, is_deterministic, is_valid};
pub use error::Error;
#[cfg(all(feature = "std", feature = "getrandom"))]
pub use generate::generate;
pub use generate::generate_with;
pub use monotonic::Monotonic;
pub use options::{Case, Options, Size};
pub use source::ByteSource;
#[cfg(feature = "getrandom")]
pub use source::GetrandomSource;

/// The README's Rust examples, collected only by `cargo test --doc` so that
/// they compile and run as doctests. They call `generate` and `decode`, so
/// they run when the `std` and `getrandom` features are both on.
#[cfg(all(doctest, feature = "std", feature = "getrandom"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(test)]
mod tests {
    // Sabotage: with `extern crate alloc;` deleted, this test no longer compiles.
    #[test]
    fn links_alloc_without_std() {
        let prefix = alloc::string::String::from("usr");
        assert_eq!(prefix.len(), 3);
    }
}
