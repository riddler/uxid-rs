//! Deterministic generation against the conformance corpus's deterministic
//! cases and its deterministic generate refusal (uxid-ex `v2.10.0`,
//! `conformance/deterministic.json` and `conformance/invalid.json`), each
//! named by its case id. Every case file runs in full once the corpus is
//! vendored.

use uxid::{ByteSource, Case, Error, Options, Size, generate_with};

/// A byte source that must never be called: a deterministic id has no
/// random field.
struct Unreachable;

impl ByteSource for Unreachable {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        panic!("the byte source was called");
    }
}

struct Deterministic {
    id: &'static str,
    prefix: Option<&'static str>,
    from: &'static str,
    size: Size,
    case: Case,
    expected: &'static str,
}

const fn case(
    id: &'static str,
    (prefix, from, size, case): (Option<&'static str>, &'static str, Size, Case),
    expected: &'static str,
) -> Deterministic {
    Deterministic {
        id,
        prefix,
        from,
        size,
        case,
        expected,
    }
}

const ALICE: &str = "alice@example.com";

/// Every line of `deterministic.json` at the tag, in its order.
const DETERMINISTIC: &[Deterministic] = &[
    case(
        "deterministic/golden-usr-xl",
        (Some("usr"), ALICE, Size::Xl, Case::Lower),
        "usr_zcvt7epac0t1ebcsjfyf7cwz25",
    ),
    case(
        "deterministic/golden-cus-s",
        (Some("cus"), "acme-corp", Size::S, Case::Lower),
        "cus_zt9vnyfd85bff0",
    ),
    case(
        "deterministic/golden-no-prefix-m",
        (None, ALICE, Size::M, Case::Lower),
        "zqz4cwzgyw8ks97869",
    ),
    case(
        "deterministic/usr-xs",
        (Some("usr"), ALICE, Size::Xs, Case::Lower),
        "usr_z1yhw20vag",
    ),
    case(
        "deterministic/usr-s",
        (Some("usr"), ALICE, Size::S, Case::Lower),
        "usr_z5rtsrkhx2h4dr",
    ),
    case(
        "deterministic/usr-m",
        (Some("usr"), ALICE, Size::M, Case::Lower),
        "usr_zt8naccefxhwnrmscd",
    ),
    case(
        "deterministic/usr-l",
        (Some("usr"), ALICE, Size::L, Case::Lower),
        "usr_zwbz6myvgz7ehe3nceap5n",
    ),
    case(
        "deterministic/usr-xl",
        (Some("usr"), ALICE, Size::Xl, Case::Lower),
        "usr_zcvt7epac0t1ebcsjfyf7cwz25",
    ),
    case(
        "deterministic/empty-prefix-xl",
        (Some(""), ALICE, Size::Xl, Case::Lower),
        "z4krgegf6drn563pvqhya4tsx9",
    ),
    case(
        "deterministic/no-prefix-xl",
        (None, ALICE, Size::Xl, Case::Lower),
        "z4krgegf6drn563pvqhya4tsx9",
    ),
    case(
        "deterministic/usr-xl-upper",
        (Some("usr"), ALICE, Size::Xl, Case::Upper),
        "usr_ZCVT7EPAC0T1EBCSJFYF7CWZ25",
    ),
    case(
        "deterministic/non-ascii-from-m",
        (Some("cus"), "café", Size::M, Case::Lower),
        "cus_zfsx9se75671f9kdjv",
    ),
];

fn options_for(case: &Deterministic) -> Options<'_> {
    let options = Options::new()
        .from(case.from)
        .size(case.size)
        .case(case.case);
    match case.prefix {
        Some(prefix) => options.prefix(prefix),
        None => options,
    }
}

// Sabotage: hashing `from` without the zero byte that follows it turns every case red.
#[test]
fn deterministic_cases_match_the_corpus() {
    for case in DETERMINISTIC {
        let id = generate_with(&options_for(case), &mut Unreachable, 0)
            .unwrap_or_else(|error| panic!("{}: refused with {error}", case.id));
        assert_eq!(id, case.expected, "{}", case.id);
    }
}

// Sabotage: dropping the `rand` check on the deterministic path turns this red.
#[test]
fn fixed_rand_with_from_is_refused() {
    // invalid/generate-rand-with-from
    let rand = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let options = Options::new().prefix("usr").from(ALICE).rand(&rand);
    let refused = generate_with(&options, &mut Unreachable, 0);
    assert_eq!(refused, Err(Error::RandWithFrom));
    assert_eq!(Error::RandWithFrom.code(), "rand_with_from");
}

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
    let options = Options::new().prefix("cus").size(Size::S).from("acme-corp");
    assert_eq!(uxid::generate(&options), Ok("cus_zt9vnyfd85bff0".into()));
}

// Sabotage: printing the code alone for `RandWithFrom` turns this red.
#[test]
fn the_rand_with_from_refusal_displays_a_sentence() {
    let text = Error::RandWithFrom.to_string();
    assert!(text.ends_with(" (rand_with_from)"), "{text}");
    assert!(text.len() > " (rand_with_from)".len(), "{text}");
}
