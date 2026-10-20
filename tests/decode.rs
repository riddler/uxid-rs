//! Decoding against every decode case and every decode refusal of the
//! conformance corpus (uxid-ex `v2.10.0`, `conformance/decode.json` and the
//! `decode` cases of `conformance/invalid.json`), each named by its case id,
//! until the corpus runner reads the vendored files; plus the validity
//! checks and a sweep of malformed inputs that must never panic.

use uxid::{Decoded, Error, Size, decode_with, is_deterministic, is_valid};

/// The first decoder clock, in milliseconds, at which every compact decode
/// case holds (inclusive), as the corpus README bounds it.
const WINDOW_START: u64 = 1_699_913_600_000;
/// The last decoder clock at which every compact decode case holds: the
/// README's end, 2164577117183, is exclusive.
const WINDOW_LAST: u64 = 2_164_577_117_182;

struct Decode {
    id: &'static str,
    input: &'static str,
    prefix: Option<&'static str>,
    size: Size,
    compact: bool,
    deterministic: bool,
    time: Option<u64>,
}

const T: u64 = 1_700_000_000_000;

/// One decode case: its id, its input, then the expected prefix, size,
/// compact flag, deterministic flag and time.
const fn case(
    id: &'static str,
    input: &'static str,
    (prefix, size, compact, deterministic, time): (
        Option<&'static str>,
        Size,
        bool,
        bool,
        Option<u64>,
    ),
) -> Decode {
    Decode {
        id,
        input,
        prefix,
        size,
        compact,
        deterministic,
        time,
    }
}

/// Every case of `decode.json`, in file order.
const DECODE: &[Decode] = &[
    case(
        "decode/xl-standard",
        "usr_01hf7yat0004hmasw9nf6yy093",
        (Some("usr"), Size::Xl, false, false, Some(T)),
    ),
    case(
        "decode/xl-compact",
        "usr_hf7yat0004hmasw9nf6yy093",
        (Some("usr"), Size::Xl, true, false, Some(T)),
    ),
    case(
        "decode/l-standard",
        "usr_01hf7yat0004hmasw9nf61",
        (Some("usr"), Size::L, false, false, Some(T)),
    ),
    case(
        "decode/l-compact",
        "usr_hf7yat0004hmasw9nf6yf",
        (Some("usr"), Size::L, true, false, Some(T)),
    ),
    case(
        "decode/m-standard",
        "usr_01hf7yat0004hmasw9",
        (Some("usr"), Size::M, false, false, Some(T)),
    ),
    case(
        "decode/m-compact-reads-as-standard",
        "usr_hf7yat0004hmasw9n3",
        (Some("usr"), Size::M, false, false, Some(51_950_139_736_068)),
    ),
    case(
        "decode/s-standard",
        "usr_01hf7yat0004h1",
        (Some("usr"), Size::S, false, false, Some(T)),
    ),
    case(
        "decode/s-compact",
        "usr_hf7yat0004hm5",
        (Some("usr"), Size::S, true, false, Some(T)),
    ),
    case(
        "decode/xs-standard",
        "usr_01hf7yat00",
        (Some("usr"), Size::Xs, false, false, Some(T)),
    ),
    case(
        "decode/xs-compact-reads-as-standard",
        "usr_hf7yat0001",
        (
            Some("usr"),
            Size::Xs,
            false,
            false,
            Some(51_950_139_736_065),
        ),
    ),
    case(
        "decode/xl-standard-upper",
        "usr_01HF7YAT0004HMASW9NF6YY093",
        (Some("usr"), Size::Xl, false, false, Some(T)),
    ),
    case(
        "decode/xl-standard-no-prefix",
        "01hf7yat0004hmasw9nf6yy093",
        (None, Size::Xl, false, false, Some(T)),
    ),
    case(
        "decode/s-standard-prefix-with-delimiter",
        "a_b_01hf7yat000001",
        (Some("a_b"), Size::S, false, false, Some(T)),
    ),
    case(
        "decode/s-compact-guard-last-valid",
        "usr_yzzzzzzz04hm5",
        (Some("usr"), Size::S, true, false, Some(1_065_151_889_407)),
    ),
    case(
        "decode/deterministic-usr-xl",
        "usr_zcvt7epac0t1ebcsjfyf7cwz25",
        (Some("usr"), Size::Xl, false, true, None),
    ),
    case(
        "decode/deterministic-cus-s",
        "cus_zt9vnyfd85bff0",
        (Some("cus"), Size::S, false, true, None),
    ),
    case(
        "decode/deterministic-no-prefix-m",
        "zqz4cwzgyw8ks97869",
        (None, Size::M, false, true, None),
    ),
    case(
        "decode/deterministic-upper-marker",
        "usr_ZCVT7EPAC0T1EBCSJFYF7CWZ25",
        (Some("usr"), Size::Xl, false, true, None),
    ),
];

/// Every `decode` case of `invalid.json`, in file order: the case id, the
/// input and the reason it is refused with.
const REFUSALS: &[(&str, &str, Error)] = &[
    ("invalid/decode-empty-string", "", Error::Empty),
    ("invalid/decode-ends-in-delimiter", "usr_", Error::Empty),
    (
        "invalid/decode-prefix-with-delimiter-ends-in-delimiter",
        "a_b_",
        Error::Empty,
    ),
    ("invalid/decode-delimiter-only", "_", Error::Empty),
    (
        "invalid/decode-length-11",
        "usr_01hf7yat000",
        Error::InvalidLength,
    ),
    (
        "invalid/decode-length-25",
        "usr_01hf7yat00000g40r40m30e20",
        Error::InvalidLength,
    ),
    (
        "invalid/decode-length-27",
        "usr_01hf7yat00000g40r40m30e2090",
        Error::InvalidLength,
    ),
    (
        "invalid/decode-no-delimiter-short",
        "usr",
        Error::InvalidLength,
    ),
    (
        "invalid/decode-character-in-time",
        "usr_01hu7yat00000g40r40m30e209",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-character-in-random",
        "usr_01hf7yat00000g4lr40m30e209",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-upper-o-in-random",
        "usr_01hf7yat00000g40r40mO0e209",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-hyphen-in-body",
        "usr_01hf7yat00-00g40r40m30e209",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-character-in-deterministic",
        "usr_zcvt7epac0t1ebcsjfyf7cwzi5",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-non-ascii-table-length",
        "01hf7yat00000g40r40m30e2\u{e9}",
        Error::InvalidCharacter,
    ),
    (
        "invalid/decode-non-ascii-off-table-length",
        "01hf7yat00000g40r40m30e20\u{e9}",
        Error::InvalidLength,
    ),
    (
        "invalid/decode-length-and-character",
        "usr_01hf7yat00u",
        Error::InvalidLength,
    ),
];

fn check(case: &Decode, decoded: &Decoded, now: u64) {
    let at = format!("{} at {now}", case.id);
    assert_eq!(decoded.prefix(), case.prefix, "{at}: prefix");
    assert_eq!(decoded.size(), case.size, "{at}: size");
    assert_eq!(decoded.is_compact(), case.compact, "{at}: compact");
    assert_eq!(
        decoded.is_deterministic(),
        case.deterministic,
        "{at}: deterministic"
    );
    assert_eq!(decoded.time(), case.time, "{at}: time");
}

// Sabotage: reading the standard time's first symbol whole (no `& 7`) turns this red.
#[test]
fn decode_cases_match_the_corpus_at_both_ends_of_the_clock_window() {
    for now in [WINDOW_START, T, WINDOW_LAST] {
        for case in DECODE {
            let decoded = decode_with(case.input, "_", now)
                .unwrap_or_else(|error| panic!("{} at {now}: {error}", case.id));
            check(case, &decoded, now);
        }
    }
}

// Sabotage: checking the alphabet before the length turns the length-and-character case red.
#[test]
fn decode_refusals_match_the_corpus() {
    for (id, input, reason) in REFUSALS {
        assert_eq!(decode_with(input, "_", T), Err(reason.clone()), "{id}");
    }
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
    let s_compact = &DECODE[7];
    assert_eq!(s_compact.id, "decode/s-compact");
    let at_day = decode_with(s_compact.input, "_", T - 86_400_000).unwrap();
    assert_eq!(at_day.time(), Some(T));
    // One millisecond earlier, the compact time lands in the window before.
    let before = decode_with(s_compact.input, "_", WINDOW_START - 1).unwrap();
    assert_eq!(before.time(), Some(T - (1 << 40)));
    // At the README's exclusive end, the guard case is no longer recovered.
    let guard = &DECODE[13];
    assert_eq!(guard.id, "decode/s-compact-guard-last-valid");
    let after = decode_with(guard.input, "_", WINDOW_LAST + 1).unwrap();
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
    for case in DECODE {
        assert!(is_valid(case.input), "{}", case.id);
    }
    for (id, input, _) in REFUSALS {
        assert!(!is_valid(input), "{id}");
    }
}

// Sabotage: an `is_deterministic` that reads the first body byte without decoding turns this red.
#[test]
fn is_deterministic_is_a_well_formed_marker_body() {
    for case in DECODE {
        assert_eq!(
            is_deterministic(case.input),
            case.deterministic,
            "{}",
            case.id
        );
    }
    assert!(!is_deterministic("usr_zcvt7epac0t1ebcsjfyf7cwzi5"));
    assert!(!is_deterministic("usr_zcvt"));
    assert!(!is_deterministic(""));
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
    for case in DECODE {
        let input = case.input;
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
