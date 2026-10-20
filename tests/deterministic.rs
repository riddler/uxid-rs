//! Deterministic generation beyond the conformance corpus's deterministic
//! cases and its deterministic generate refusal, which `tests/corpus.rs`
//! runs: the options a deterministic id does not read, the delimiter, the
//! prefix in the hash, `generate` and the refusal's sentence.

use uxid::{ByteSource, Error, Options, generate_with};

/// A byte source that must never be called: a deterministic id has no
/// random field.
struct Unreachable;

impl ByteSource for Unreachable {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        panic!("the byte source was called");
    }
}

const ALICE: &str = "alice@example.com";

// Sabotage: reading `time` on the deterministic path (refusing it above 2^48 - 1) turns this red.
#[test]
fn time_compact_time_rand_size_and_now_are_not_read() {
    let plain = Options::new().prefix("usr").from(ALICE);
    let expected = "usr_zcvt7epac0t1ebcsjfyf7cwz25";
    let ignored = plain
        .clone()
        .time(u64::MAX)
        .compact_time(true)
        .rand_size(11);
    assert_eq!(
        generate_with(&ignored, &mut Unreachable, u64::MAX),
        Ok(expected.into())
    );
    assert_eq!(
        generate_with(&plain, &mut Unreachable, 0),
        Ok(expected.into())
    );
}

// Sabotage: writing `_` in place of the delimiter option on the deterministic path turns this red.
#[test]
fn the_delimiter_applies_and_is_not_hashed() {
    let options = Options::new().prefix("usr").from(ALICE).delimiter("-");
    assert_eq!(
        generate_with(&options, &mut Unreachable, 0),
        Ok("usr-zcvt7epac0t1ebcsjfyf7cwz25".into())
    );
    let empty = Options::new().prefix("usr").from(ALICE).delimiter("");
    assert_eq!(
        generate_with(&empty, &mut Unreachable, 0),
        Err(Error::EmptyDelimiter)
    );
}

// Sabotage: leaving the prefix out of the hash input turns this red.
#[test]
fn the_prefix_is_part_of_the_hash() {
    let usr = generate_with(
        &Options::new().prefix("usr").from(ALICE),
        &mut Unreachable,
        0,
    );
    let cus = generate_with(
        &Options::new().prefix("cus").from(ALICE),
        &mut Unreachable,
        0,
    );
    assert_eq!(usr.unwrap()["usr_".len()..], *"zcvt7epac0t1ebcsjfyf7cwz25");
    assert_ne!(cus.unwrap()["cus_".len()..], *"zcvt7epac0t1ebcsjfyf7cwz25");
}

// Sabotage: a `generate_with` that ignores `from` turns this red.
#[cfg(all(feature = "std", feature = "getrandom"))]
#[test]
fn generate_builds_the_deterministic_id_too() {
    let options = Options::new()
        .prefix("cus")
        .size(uxid::Size::S)
        .from("acme-corp");
    assert_eq!(uxid::generate(&options), Ok("cus_zt9vnyfd85bff0".into()));
}

// Sabotage: printing the code alone for `RandWithFrom` turns this red.
#[test]
fn the_rand_with_from_refusal_displays_a_sentence() {
    let text = Error::RandWithFrom.to_string();
    assert!(text.ends_with(" (rand_with_from)"), "{text}");
    assert!(text.len() > " (rand_with_from)".len(), "{text}");
}
