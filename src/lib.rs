//! Prefixed, K-sortable identifiers a person can read, wire-compatible with
//! the Elixir [uxid](https://github.com/riddler/uxid-ex) package.
//!
//! The crate is `no_std` with `alloc`. It exports nothing yet.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(test)]
mod tests {
    // Sabotage: with `extern crate alloc;` deleted, this test no longer compiles.
    #[test]
    fn links_alloc_without_std() {
        let prefix = alloc::string::String::from("usr");
        assert_eq!(prefix.len(), 3);
    }
}
