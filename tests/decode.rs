//! Decoding beyond the conformance corpus's cases, which `tests/corpus.rs`
//! runs: the clock window's ends, the validity checks over the vendored
//! decode inputs and refusals, and a sweep of malformed inputs, built from
//! the vendored decode inputs, that must never panic.

use std::fs;

use serde_json::Value;
use uxid::{
    Error, Size, decode_with, is_deterministic, is_deterministic_with, is_valid, is_valid_with,
};

/// The first decoder clock, in milliseconds, at which every compact decode
/// case holds (inclusive), as the corpus README bounds it.
const WINDOW_START: u64 = 1_699_913_600_000;
/// The last decoder clock at which every compact decode case holds: the
/// README's end, 2164577117183, is exclusive.
const WINDOW_LAST: u64 = 2_164_577_117_182;

const T: u64 = 1_700_000_000_000;

/// The cases of a vendored case file, one JSON object per line.
fn cases(file: &str) -> Vec<Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/conformance/");
    fs::read_to_string(format!("{path}{file}"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// Every decode case's id, input and deterministic flag, in file order.
fn decode_inputs() -> Vec<(String, String, bool)> {
    cases("decode.json")
        .iter()
        .map(|case| {
            (
                case["id"].as_str().unwrap().to_owned(),
                case["input"].as_str().unwrap().to_owned(),
                case["expected"]["deterministic"].as_bool().unwrap(),
            )
        })
        .collect()
}

/// Every decode case's id, input and expected prefix (`None` for the
/// corpus's null), in file order.
fn decode_prefixes() -> Vec<(String, String, Option<String>)> {
    cases("decode.json")
        .iter()
        .map(|case| {
            (
                case["id"].as_str().unwrap().to_owned(),
                case["input"].as_str().unwrap().to_owned(),
                case["expected"]["prefix"].as_str().map(str::to_owned),
            )
        })
        .collect()
}

/// `input` with its last `_` replaced by `delimiter`, or unchanged when it
/// has none.
fn redelimit(input: &str, delimiter: &str) -> String {
    match input.rfind('_') {
        Some(at) => format!("{}{delimiter}{}", &input[..at], &input[at + 1..]),
        None => input.to_owned(),
    }
}

/// The input of the decode case `id`.
fn decode_input(id: &str) -> String {
    decode_inputs()
        .into_iter()
        .find(|(case, _, _)| case == id)
        .unwrap_or_else(|| panic!("no decode case {id}"))
        .1
}

/// Every `decode` refusal of `invalid.json`: its id and its input.
fn refused_inputs() -> Vec<(String, String)> {
    cases("invalid.json")
        .iter()
        .filter_map(|case| {
            let input = case["input"]["decode"].as_str()?;
            Some((case["id"].as_str().unwrap().to_owned(), input.to_owned()))
        })
        .collect()
}

// Sabotage: a refusal code spelled `invalid_chars` turns this red.
#[test]
fn the_refusal_codes_are_the_corpus_names() {
    assert_eq!(Error::Empty.code(), "empty");
    assert_eq!(Error::InvalidLength.code(), "invalid_length");
    assert_eq!(Error::InvalidCharacter.code(), "invalid_character");
    for error in [Error::Empty, Error::InvalidLength, Error::InvalidCharacter] {
        let text = error.to_string();
        assert!(text.ends_with(&format!("({})", error.code())), "{text}");
        assert!(text.len() > error.code().len() + 3, "{text}");
    }
}

// Sabotage: subtracting the window when the result is exactly one day ahead (`>=`) turns this red.
#[test]
fn the_clock_window_ends_are_tight() {
    // A clock one day before 1700000000000 still reads it in its own window.
    let s_compact = decode_input("decode/s-compact");
    let at_day = decode_with(&s_compact, "_", T - 86_400_000).unwrap();
    assert_eq!(at_day.time(), Some(T));
    // One millisecond earlier, the compact time lands in the window before.
    let before = decode_with(&s_compact, "_", WINDOW_START - 1).unwrap();
    assert_eq!(before.time(), Some(T - (1 << 40)));
    // At the README's exclusive end, the guard case is no longer recovered.
    let guard = decode_input("decode/s-compact-guard-last-valid");
    let after = decode_with(&guard, "_", WINDOW_LAST + 1).unwrap();
    assert_eq!(after.time(), Some(1_065_151_889_407 + (1 << 40)));
}

// Sabotage: subtracting the window (wrapping) whenever the result is a day ahead, with no window before it, turns this red.
#[test]
fn a_compact_time_decoded_near_the_epoch_is_not_negative() {
    let decoded = decode_with("usr_hf7yat0004hm5", "_", 0).unwrap();
    assert_eq!(decoded.time(), Some(T - (1 << 40)));
    let decoded = decode_with("usr_hf7yat0004hm5", "_", 86_400_000).unwrap();
    assert_eq!(decoded.time(), Some(T - (1 << 40)));
}

// Sabotage: reading every non-deterministic body's time as a compact one against `now` turns this red.
#[test]
fn a_standard_or_deterministic_body_does_not_read_the_clock() {
    for now in [0, T, u64::MAX] {
        assert_eq!(
            decode_with("usr_01hf7yat0004h1", "_", now).unwrap().time(),
            Some(T)
        );
        assert_eq!(
            decode_with("cus_zt9vnyfd85bff0", "_", now).unwrap().time(),
            None
        );
    }
}

// Sabotage: dropping the `time > now` guard before `time - now` (an overflow panic for a clock ahead of the id) turns this red.
#[test]
fn any_clock_gives_an_answer() {
    for now in [0, 1, (1 << 40) - 1, 1 << 40, 1 << 48, u64::MAX] {
        for input in [
            "usr_hf7yat0004hm5",
            "usr_yzzzzzzz04hm5",
            "usr_0000000004hm5",
        ] {
            let outcome = std::panic::catch_unwind(|| decode_with(input, "_", now));
            assert!(
                outcome.is_ok_and(|result| result.is_ok()),
                "{input} at {now}"
            );
        }
    }
}

// Sabotage: splitting at the last occurrence of the delimiter (`rfind`) turns this red.
#[test]
fn the_split_follows_the_left_to_right_scan_for_a_delimiter_that_overlaps_itself() {
    // Under `--`, the scan matches at 3 and resumes at 5, so the body is
    // `-01hf7yat00`, 11 bytes: the reference's split, not the last occurrence.
    // The guide's wording ("the last occurrence") reads otherwise for such a
    // delimiter, and no corpus case fixes it; the reference decides here.
    assert_eq!(
        decode_with("usr---01hf7yat00", "--", T),
        Err(Error::InvalidLength)
    );
    let decoded = decode_with("usr--01hf7yat00", "--", T).unwrap();
    assert_eq!(decoded.prefix(), Some("usr"));
    let decoded = decode_with("a--b--01hf7yat00", "--", T).unwrap();
    assert_eq!(decoded.prefix(), Some("a--b"));
}

// Sabotage: reporting `None` for an id that starts with the delimiter turns this red.
#[test]
fn an_id_that_starts_with_the_delimiter_has_an_empty_prefix() {
    let decoded = decode_with("_01hf7yat00", "_", T).unwrap();
    assert_eq!(decoded.prefix(), Some(""));
    assert_eq!(decoded.time(), Some(T));
}

// Sabotage: resetting the compact flag for a deterministic body turns this red.
#[test]
fn a_marker_body_at_a_compact_length_decodes_as_deterministic_and_compact() {
    // As the reference decodes it: the length gives the form, the marker
    // makes it deterministic, and there is no time.
    let decoded = decode_with("usr_zcvt7epac0t1e", "_", T).unwrap();
    assert_eq!(decoded.size(), Size::S);
    assert!(decoded.is_compact());
    assert!(decoded.is_deterministic());
    assert_eq!(decoded.time(), None);
}

// Sabotage: dropping the empty-delimiter check turns this red.
#[test]
fn an_empty_delimiter_is_refused() {
    assert_eq!(
        decode_with("usr_01hf7yat00", "", T),
        Err(Error::EmptyDelimiter)
    );
}

// Sabotage: an `is_valid` that ignores the length rule turns this red.
#[test]
fn is_valid_is_whether_the_id_decodes() {
    for (id, input, _) in decode_inputs() {
        assert!(is_valid(&input), "{id}");
    }
    for (id, input) in refused_inputs() {
        assert!(!is_valid(&input), "{id}");
    }
}

// Sabotage: an `is_deterministic` that reads the first body byte without decoding turns this red.
#[test]
fn is_deterministic_is_a_well_formed_marker_body() {
    for (id, input, deterministic) in decode_inputs() {
        assert_eq!(is_deterministic(&input), deterministic, "{id}");
    }
    assert!(!is_deterministic("usr_zcvt7epac0t1ebcsjfyf7cwzi5"));
    assert!(!is_deterministic("usr_zcvt"));
    assert!(!is_deterministic(""));
}

// Sabotage: an `is_valid_with` that splits on `_` whatever the delimiter turns this red.
#[test]
fn is_valid_with_is_whether_the_id_decodes_under_its_delimiter() {
    for (id, input, _) in decode_inputs() {
        assert!(is_valid_with(&input, "_", None), "{id}");
        assert!(is_valid_with(&redelimit(&input, "--"), "--", None), "{id}");
    }
    for (id, input) in refused_inputs() {
        assert!(!is_valid_with(&input, "_", None), "{id}");
        assert!(!is_valid_with(&redelimit(&input, "--"), "--", None), "{id}");
    }
    assert!(!is_valid_with("usr_01hf7yat00", "--", None));
}

// Sabotage: an `is_valid_with` that ignores the required prefix turns this red.
#[test]
fn is_valid_with_requires_exactly_the_prefix_given() {
    for (id, input, prefix) in decode_prefixes() {
        assert!(is_valid_with(&input, "_", Some("")), "{id}");
        assert!(!is_valid_with(&input, "_", Some("org")), "{id}");
        match prefix {
            Some(prefix) => {
                assert!(is_valid_with(&input, "_", Some(&prefix)), "{id}");
                let redelimited = redelimit(&input, "--");
                assert!(is_valid_with(&redelimited, "--", Some(&prefix)), "{id}");
                assert!(!is_valid_with(&input, "_", Some(&prefix[1..])), "{id}");
            }
            None => assert!(!is_valid_with(&input, "_", Some("usr")), "{id}"),
        }
    }
    for (id, input) in refused_inputs() {
        assert!(!is_valid_with(&input, "_", Some("usr")), "{id}");
    }
}

// Sabotage: an `is_deterministic_with` that splits on `_` whatever the delimiter turns this red.
#[test]
fn is_deterministic_with_is_a_well_formed_marker_body_under_its_delimiter() {
    for (id, input, deterministic) in decode_inputs() {
        assert_eq!(is_deterministic_with(&input, "_"), deterministic, "{id}");
        let redelimited = redelimit(&input, "--");
        assert_eq!(
            is_deterministic_with(&redelimited, "--"),
            deterministic,
            "{id}"
        );
    }
    for (id, input) in refused_inputs() {
        assert!(!is_deterministic_with(&input, "_"), "{id}");
        assert!(
            !is_deterministic_with(&redelimit(&input, "--"), "--"),
            "{id}"
        );
    }
    assert!(!is_deterministic_with(
        "usr--zcvt7epac0t1ebcsjfyf7cwz25",
        "_"
    ));
}

// Sabotage: an `is_valid_with` that falls back to `_` for an empty delimiter turns this red.
#[test]
fn an_empty_delimiter_makes_every_check_false() {
    for (id, input, _) in decode_inputs() {
        assert!(!is_valid_with(&input, "", None), "{id}");
        assert!(!is_deterministic_with(&input, ""), "{id}");
    }
}

// Sabotage: a `decode` that splits on another delimiter than `_` turns this red.
#[cfg(feature = "std")]
#[test]
fn decode_reads_the_underscore_and_the_system_clock() {
    for input in [
        "usr_01hf7yat0004h1",
        "a_b_01hf7yat000001",
        "cus_zt9vnyfd85bff0",
    ] {
        let decoded = uxid::decode(input);
        assert_eq!(decoded, decode_with(input, "_", T), "{input}");
        assert!(decoded.is_ok(), "{input}");
    }
    // The system clock is inside the corpus's window until mid-2038.
    let compact = uxid::decode("usr_hf7yat0004hm5");
    assert_eq!(compact.map(|decoded| decoded.time()), Ok(Some(T)));
    assert_eq!(uxid::decode("usr_"), Err(Error::Empty));
}

/// Inputs that are truncated, overlong, out of the alphabet, non-ASCII or
/// empty, built from the well-formed decode cases.
fn malformed() -> Vec<String> {
    let mut inputs = vec![
        String::new(),
        "_".to_owned(),
        "__".to_owned(),
        "usr_".to_owned(),
        "\u{e9}".to_owned(),
        "usr_\u{1f600}".to_owned(),
        "\u{1f600}".repeat(26),
        "\u{20ac}".repeat(8) + "01",
        "usr_\0\0\0\0\0\0\0\0\0\0".to_owned(),
    ];
    let wrong = [
        "i",
        "I",
        "l",
        "L",
        "o",
        "O",
        "u",
        "U",
        "-",
        " ",
        "\u{e9}",
        "\u{20ac}",
        "\u{1f600}",
        "\u{ff10}",
    ];
    for (_, input, _) in decode_inputs() {
        let input = input.as_str();
        for end in (0..input.len()).filter(|&end| input.is_char_boundary(end)) {
            inputs.push(input[..end].to_owned());
            inputs.push(input[end..].to_owned());
        }
        for extra in ["0", "z", "00", "\u{e9}", "_", "000000000000000000000000000"] {
            inputs.push(format!("{input}{extra}"));
        }
        for (at, _) in input.char_indices() {
            for symbol in wrong {
                let mut changed = String::from(&input[..at]);
                changed.push_str(symbol);
                changed.push_str(&input[at + 1..]);
                inputs.push(changed);
            }
        }
    }
    inputs
}

// Sabotage: in `refuse`, answering `Ok(form)` without the alphabet check turns this red.
#[test]
fn malformed_input_never_panics() {
    let inputs = malformed();
    assert!(inputs.len() > 1_000);
    let mut refused = 0;
    for input in &inputs {
        for now in [0, T, u64::MAX] {
            let outcome = std::panic::catch_unwind(|| decode_with(input, "_", now));
            let Ok(result) = outcome else {
                panic!("decode panicked on {input:?}");
            };
            match result {
                Ok(decoded) => {
                    // An independent oracle: the body is a table length of
                    // alphabet bytes.
                    let body = input.rsplit('_').next().unwrap_or(input);
                    assert!(
                        [10, 13, 14, 18, 21, 22, 24, 26].contains(&body.len()),
                        "{input:?} {decoded:?}"
                    );
                    assert!(
                        body.bytes()
                            .all(|b| b.is_ascii_alphanumeric() && !b"iIlLoOuU".contains(&b)),
                        "{input:?} {decoded:?}"
                    );
                }
                Err(error) => {
                    assert!(
                        matches!(
                            error,
                            Error::Empty | Error::InvalidLength | Error::InvalidCharacter
                        ),
                        "{input:?}: {error}"
                    );
                    assert!(!is_valid(input), "{input:?}");
                    assert!(!is_deterministic(input), "{input:?}");
                    refused += 1;
                }
            }
        }
    }
    assert!(refused > 0);
}
