//! Every case of the conformance corpus vendored under `conformance/`
//! (uxid-ex at the tag `conformance/SOURCE.json` names), run by its kind as
//! the corpus README's "Running the cases" says.
//!
//! The case files are read through `manifest.json`, so every file and every
//! line it lists is run and nothing is skipped. A case is read strictly: a
//! key the runner does not know, a missing key or a value of the wrong type
//! fails the case, so a later corpus cannot be half-read. A failing case is
//! reported by its id. `tests/corpus_hash.rs` checks the files' hash.

use std::collections::BTreeSet;
use std::fs;

use serde_json::{Map, Value};
use uxid::{ByteSource, Case, Error, Monotonic, Options, Size, decode_with, generate_with};

/// The vendored corpus directory.
const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/conformance");

/// The corpus shape this runner knows (`corpus_version` in the manifest).
const CORPUS_VERSION: u64 = 1;

/// The time passed as `now` to a generate call: no encode or deterministic
/// case reads it, and an invalid case is refused whatever it is.
const NOW: u64 = 1_700_000_000_000;

/// The first decoder clock at which every compact decode case holds
/// (inclusive), as the corpus README bounds it.
const WINDOW_START: u64 = 1_699_913_600_000;
/// The last decoder clock at which every compact decode case holds: the
/// README's end, 2164577117183, is exclusive.
const WINDOW_LAST: u64 = 2_164_577_117_182;

/// What a failing case reports: why it failed.
type Outcome = Result<(), String>;

/// The object `value` holds, after checking that it has every key in
/// `required` and no key outside `required` and `optional`.
fn object<'a>(
    value: &'a Value,
    required: &[&str],
    optional: &[&str],
) -> Result<&'a Map<String, Value>, String> {
    let map = value
        .as_object()
        .ok_or_else(|| format!("not an object: {value}"))?;
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("missing key {key:?}"));
        }
    }
    for key in map.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("unknown key {key:?}"));
        }
    }
    Ok(map)
}

fn field<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a Value, String> {
    map.get(key).ok_or_else(|| format!("missing key {key:?}"))
}

fn string<'a>(map: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    field(map, key)?
        .as_str()
        .ok_or_else(|| format!("{key:?} is not a string"))
}

/// A string or `null`.
fn nullable_string<'a>(map: &'a Map<String, Value>, key: &str) -> Result<Option<&'a str>, String> {
    match field(map, key)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        other => Err(format!("{key:?} is not a string or null: {other}")),
    }
}

fn integer(value: &Value, key: &str) -> Result<u64, String> {
    value
        .as_u64()
        .ok_or_else(|| format!("{key:?} is not a non-negative integer: {value}"))
}

fn boolean(map: &Map<String, Value>, key: &str) -> Result<bool, String> {
    field(map, key)?
        .as_bool()
        .ok_or_else(|| format!("{key:?} is not a boolean"))
}

/// A size, in the short spelling the corpus uses.
fn size(map: &Map<String, Value>, key: &str) -> Result<Size, String> {
    match string(map, key)? {
        "xs" => Ok(Size::Xs),
        "s" => Ok(Size::S),
        "m" => Ok(Size::M),
        "l" => Ok(Size::L),
        "xl" => Ok(Size::Xl),
        other => Err(format!("{key:?} is not a size: {other:?}")),
    }
}

fn letter_case(map: &Map<String, Value>, key: &str) -> Result<Case, String> {
    match string(map, key)? {
        "lower" => Ok(Case::Lower),
        "upper" => Ok(Case::Upper),
        other => Err(format!("{key:?} is not a case: {other:?}")),
    }
}

/// Lowercase hex to bytes.
fn hex(map: &Map<String, Value>, key: &str) -> Result<Vec<u8>, String> {
    let text = string(map, key)?;
    let well_formed = text.len() % 2 == 0
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !well_formed {
        return Err(format!("{key:?} is not lowercase hex: {text:?}"));
    }
    Ok((0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect())
}

/// A byte source for a request whose random field is fixed or absent: it
/// fails if it is called, so a call shows as a failing case.
struct NotCalled;

impl ByteSource for NotCalled {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        Err(Error::ByteSource)
    }
}

/// A byte source for a request the case says is refused: it writes zeros,
/// so a request that is not refused builds an id and the case fails on it.
struct Zeros;

impl ByteSource for Zeros {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        buf.fill(0);
        Ok(())
    }
}

/// Checks `actual` against `expected`, naming `what` on a difference.
fn same<T: PartialEq + core::fmt::Debug>(what: &str, actual: T, expected: T) -> Outcome {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: got {actual:?}, expected {expected:?}"))
    }
}

/// Builds the options an encode case, or an invalid case's generate
/// request, spells: an absent option takes its default, and a prefix
/// written `null` is no prefix.
fn options<'a>(
    input: &'a Map<String, Value>,
    rand: &'a Option<Vec<u8>>,
) -> Result<Options<'a>, String> {
    let mut options = Options::new();
    if input.contains_key("prefix") {
        if let Some(prefix) = nullable_string(input, "prefix")? {
            options = options.prefix(prefix);
        }
    }
    if input.contains_key("size") {
        options = options.size(size(input, "size")?);
    }
    if input.contains_key("compact") {
        options = options.compact_time(boolean(input, "compact")?);
    }
    if input.contains_key("case") {
        options = options.case(letter_case(input, "case")?);
    }
    if let Some(time) = input.get("time") {
        options = options.time(integer(time, "time")?);
    }
    if let Some(width) = input.get("rand_size") {
        let width = usize::try_from(integer(width, "rand_size")?).map_err(|e| e.to_string())?;
        options = options.rand_size(width);
    }
    if let Some(bytes) = rand {
        options = options.rand(bytes);
    }
    if input.contains_key("from") {
        options = options.from(string(input, "from")?);
    }
    Ok(options)
}

/// `encode`: the id built from the input, with the random field fixed to
/// `rand`, is `expected`.
fn run_encode(case: &Map<String, Value>) -> Outcome {
    let input = object(
        field(case, "input")?,
        &["prefix", "size", "compact", "case", "time", "rand"],
        &[],
    )?;
    let expected = string(case, "expected")?;
    let rand = Some(hex(input, "rand")?);
    let id = generate_with(&options(input, &rand)?, &mut NotCalled, NOW)
        .map_err(|error| format!("refused with {error}"))?;
    same("id", id.as_str(), expected)
}

/// `deterministic`: the deterministic id of `from` under the prefix, size
/// and case is `expected`.
fn run_deterministic(case: &Map<String, Value>) -> Outcome {
    let input = object(
        field(case, "input")?,
        &["prefix", "from", "size", "case"],
        &[],
    )?;
    let expected = string(case, "expected")?;
    let id = generate_with(&options(input, &None)?, &mut NotCalled, NOW)
        .map_err(|error| format!("refused with {error}"))?;
    same("id", id.as_str(), expected)
}

/// `decode`: the input decodes to the expected prefix, size, compact flag,
/// deterministic flag and time, for a decoder clock at both ends of the
/// README's window and inside it.
fn run_decode(case: &Map<String, Value>) -> Outcome {
    let input = string(case, "input")?;
    let expected = object(
        field(case, "expected")?,
        &["prefix", "size", "compact", "deterministic", "time"],
        &[],
    )?;
    let prefix = nullable_string(expected, "prefix")?;
    let size = size(expected, "size")?;
    let compact = boolean(expected, "compact")?;
    let deterministic = boolean(expected, "deterministic")?;
    let time = match field(expected, "time")? {
        Value::Null => None,
        value => Some(integer(value, "time")?),
    };
    for now in [WINDOW_START, NOW, WINDOW_LAST] {
        let decoded = decode_with(input, "_", now)
            .map_err(|error| format!("at {now}: refused with {error}"))?;
        same(&format!("prefix at {now}"), decoded.prefix(), prefix)?;
        same(&format!("size at {now}"), decoded.size(), size)?;
        same(&format!("compact at {now}"), decoded.is_compact(), compact)?;
        same(
            &format!("deterministic at {now}"),
            decoded.is_deterministic(),
            deterministic,
        )?;
        same(&format!("time at {now}"), decoded.time(), time)?;
    }
    Ok(())
}

/// `invalid`: the string is refused on decode, or the generate request is
/// refused, with the error `expected_error` names.
fn run_invalid(case: &Map<String, Value>) -> Outcome {
    let expected = string(case, "expected_error")?;
    let input = object(field(case, "input")?, &[], &["decode", "generate"])?;
    let refused = match (input.get("decode"), input.get("generate")) {
        (Some(_), None) => match decode_with(string(input, "decode")?, "_", NOW) {
            Ok(decoded) => return Err(format!("decoded to {decoded:?}")),
            Err(error) => error,
        },
        (None, Some(request)) => {
            let request = object(
                request,
                &[],
                &[
                    "prefix",
                    "size",
                    "compact",
                    "case",
                    "time",
                    "rand",
                    "rand_size",
                    "from",
                    "monotonic",
                ],
            )?;
            match refused_request(request)? {
                Ok(id) => return Err(format!("built {id:?}")),
                Err(error) => error,
            }
        }
        _ => return Err("input holds neither decode nor generate, or both".into()),
    };
    same("error", refused.code(), expected)
}

/// Makes an invalid case's generate request: with `monotonic` true, a call
/// to a monotonic generator built from the request's options.
fn refused_request(request: &Map<String, Value>) -> Result<Result<String, Error>, String> {
    let rand = match request.contains_key("rand") {
        true => Some(hex(request, "rand")?),
        false => None,
    };
    let options = options(request, &rand)?;
    let monotonic = match request.contains_key("monotonic") {
        true => boolean(request, "monotonic")?,
        false => false,
    };
    if monotonic {
        Ok(Monotonic::new(&options)
            .and_then(|mut generator| generator.generate_with(&mut Zeros, NOW)))
    } else {
        Ok(generate_with(&options, &mut Zeros, NOW))
    }
}

/// The draws of a monotonic case, replayed as the bytes the generator reads
/// for each: the first call's draw is `initial_seed`; a later call's first
/// draw is the next of `steps`, written as `step - 1`; the overflowing call's
/// second draw is `overflow_seed`. Each value is written big-endian in the
/// bytes asked for, and one that does not fit them is a failure.
struct Replay {
    initial_seed: Option<u64>,
    steps: Vec<u64>,
    overflow_seed: Option<u64>,
    call: usize,
    draws_in_call: usize,
    problem: Option<String>,
}

impl Replay {
    /// Marks the start of a generate call.
    fn start_call(&mut self, call: usize) {
        self.call = call;
        self.draws_in_call = 0;
    }

    fn spent(&self) -> bool {
        self.initial_seed.is_none() && self.steps.is_empty() && self.overflow_seed.is_none()
    }

    fn next_value(&mut self) -> Result<u64, String> {
        let draw = self.draws_in_call;
        self.draws_in_call += 1;
        match (self.call, draw) {
            (0, 0) => self
                .initial_seed
                .take()
                .ok_or_else(|| "initial_seed drawn twice".to_owned()),
            (_, 0) if !self.steps.is_empty() => {
                let step = self.steps.remove(0);
                step.checked_sub(1).ok_or_else(|| "a step of 0".to_owned())
            }
            (_, 1) => self.overflow_seed.take().ok_or_else(|| {
                format!(
                    "call {}: a second draw with no overflow_seed left",
                    self.call
                )
            }),
            _ => Err(format!(
                "call {}: draw {draw} is past the case's draws",
                self.call
            )),
        }
    }
}

impl ByteSource for Replay {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let written = self.next_value().and_then(|value| {
            let bytes = u128::from(value).to_be_bytes();
            let (high, low) = bytes.split_at(bytes.len() - buf.len());
            if high.iter().any(|&b| b != 0) {
                return Err(format!("{value} does not fit {} bytes", buf.len()));
            }
            buf.copy_from_slice(low);
            Ok(())
        });
        written.map_err(|problem| {
            self.problem.get_or_insert(problem);
            Error::ByteSource
        })
    }
}

/// `monotonic`: one generator, lower case, for the prefix, size and time
/// form, its random source replaced by the case's draws, called once per
/// time, emits the expected ids in order and reads every draw.
fn run_monotonic(case: &Map<String, Value>) -> Outcome {
    let input = object(
        field(case, "input")?,
        &[
            "prefix",
            "size",
            "compact",
            "times",
            "initial_seed",
            "steps",
            "overflow_seed",
        ],
        &[],
    )?;
    let integers = |key: &str| -> Result<Vec<u64>, String> {
        field(input, key)?
            .as_array()
            .ok_or_else(|| format!("{key:?} is not an array"))?
            .iter()
            .map(|value| integer(value, key))
            .collect()
    };
    let expected: Vec<&str> = field(case, "expected")?
        .as_array()
        .ok_or("\"expected\" is not an array")?
        .iter()
        .map(|value| value.as_str().ok_or("an expected id is not a string"))
        .collect::<Result<_, _>>()?;
    let mut options = Options::new()
        .size(size(input, "size")?)
        .compact_time(boolean(input, "compact")?);
    if let Some(prefix) = nullable_string(input, "prefix")? {
        options = options.prefix(prefix);
    }
    let mut source = Replay {
        initial_seed: Some(integer(field(input, "initial_seed")?, "initial_seed")?),
        steps: integers("steps")?,
        overflow_seed: match field(input, "overflow_seed")? {
            Value::Null => None,
            value => Some(integer(value, "overflow_seed")?),
        },
        call: 0,
        draws_in_call: 0,
        problem: None,
    };
    let mut generator =
        Monotonic::new(&options).map_err(|error| format!("generator refused with {error}"))?;
    let mut ids = Vec::new();
    for (call, now) in integers("times")?.into_iter().enumerate() {
        source.start_call(call);
        match generator.generate_with(&mut source, now) {
            Ok(id) => ids.push(id),
            Err(error) => {
                let problem = source.problem.take().unwrap_or_default();
                return Err(format!("call {call} refused with {error} {problem}"));
            }
        }
    }
    same("ids", ids.iter().map(String::as_str).collect(), expected)?;
    if source.spent() {
        Ok(())
    } else {
        Err("draws left unread".into())
    }
}

/// Runs one case line of a file of `kind`: the line holds exactly `id`,
/// `input` and the expected answer, and the id starts with the kind.
fn run_case(kind: &str, line: &str) -> Result<(), (String, String)> {
    let unknown = || "(no id)".to_owned();
    let value: Value =
        serde_json::from_str(line).map_err(|error| (unknown(), format!("not JSON: {error}")))?;
    let answer = if kind == "invalid" {
        "expected_error"
    } else {
        "expected"
    };
    let case = object(&value, &["id", "input", answer], &[]).map_err(|why| (unknown(), why))?;
    let id = string(case, "id")
        .map_err(|why| (unknown(), why))?
        .to_owned();
    if !id.starts_with(&format!("{kind}/")) {
        return Err((id, format!("the id does not start with {kind}/")));
    }
    let outcome = match kind {
        "encode" => run_encode(case),
        "deterministic" => run_deterministic(case),
        "decode" => run_decode(case),
        "invalid" => run_invalid(case),
        "monotonic" => run_monotonic(case),
        other => Err(format!("a case kind this runner does not know: {other:?}")),
    };
    outcome.map_err(|why| (id, why))
}

/// Runs every case of every file the manifest lists, and returns the
/// failures (each a case id and why) and the number of cases run.
fn run_corpus() -> (Vec<(String, String)>, usize) {
    let text = fs::read_to_string(format!("{CORPUS}/manifest.json")).unwrap();
    let manifest: Value = serde_json::from_str(&text).unwrap();
    let manifest = object(&manifest, &["corpus_version", "files", "corpus_hash"], &[]).unwrap();
    let version = integer(field(manifest, "corpus_version").unwrap(), "corpus_version").unwrap();
    assert_eq!(
        version, CORPUS_VERSION,
        "a corpus version this runner does not know"
    );

    let mut failures = Vec::new();
    let mut ids = BTreeSet::new();
    let mut run = 0;
    for entry in field(manifest, "files").unwrap().as_array().unwrap() {
        let entry = object(entry, &["kind", "file", "case_count"], &[]).unwrap();
        let kind = string(entry, "kind").unwrap();
        let file = string(entry, "file").unwrap();
        let count = integer(field(entry, "case_count").unwrap(), "case_count").unwrap();
        let text = fs::read_to_string(format!("{CORPUS}/{file}")).unwrap();
        assert!(
            text.is_empty() || text.ends_with('\n'),
            "{file}: the last line has no newline"
        );
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len() as u64,
            count,
            "{file}: lines against case_count"
        );
        for line in lines {
            run += 1;
            match run_case(kind, line) {
                Ok(()) => {}
                Err(failure) => failures.push(failure),
            }
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if let Some(id) = value.get("id").and_then(Value::as_str) {
                    assert!(
                        ids.insert(id.to_owned()),
                        "{file}: the case id {id} is not unique"
                    );
                }
            }
        }
    }
    (failures, run)
}

// Sabotage: in `push_bytes`, padding the short last group shifted left turns every encode case with a partial group red.
#[test]
fn every_case_the_manifest_lists_passes() {
    let (failures, run) = run_corpus();
    let report: Vec<String> = failures
        .iter()
        .map(|(id, why)| format!("{id}: {why}"))
        .collect();
    assert!(
        report.is_empty(),
        "{} failing case(s):\n{}",
        report.len(),
        report.join("\n")
    );
    assert!(run > 0, "no case ran");
}

// Sabotage: dropping the unknown-key check in `object` turns this red.
#[test]
fn a_case_with_a_key_the_runner_does_not_know_fails() {
    let line = r#"{"expected":"usr_01hf7yat00","id":"encode/extra","input":{"case":"lower","compact":false,"delimiter":"-","prefix":"usr","rand":"","size":"xs","time":1700000000000}}"#;
    let failure = run_case("encode", line).unwrap_err();
    assert_eq!(failure.0, "encode/extra");
    assert!(
        failure.1.contains("unknown key \"delimiter\""),
        "{failure:?}"
    );

    let line = r#"{"expected":"usr_01hf7yat00","id":"encode/extra","input":{"case":"lower","compact":false,"prefix":"usr","rand":"","size":"xs","time":1700000000000},"note":"x"}"#;
    assert!(
        run_case("encode", line)
            .unwrap_err()
            .1
            .contains("unknown key \"note\"")
    );
}

// Sabotage: running an unknown kind's cases as encode cases turns this red.
#[test]
fn a_case_of_a_kind_the_runner_does_not_know_fails() {
    let line = r#"{"expected":"x","id":"future/one","input":{}}"#;
    let failure = run_case("future", line).unwrap_err();
    assert!(failure.1.contains("does not know"), "{failure:?}");
}

// Sabotage: comparing only the first expected id of a monotonic case turns this red.
#[test]
fn a_case_whose_answer_differs_fails_with_its_id() {
    let line = r#"{"expected":["hf7yat0000000","hf7yat0000801"],"id":"monotonic/wrong","input":{"compact":true,"initial_seed":0,"overflow_seed":null,"prefix":null,"size":"s","steps":[4096],"times":[1700000000000,1700000000000]}}"#;
    let failure = run_case("monotonic", line).unwrap_err();
    assert_eq!(failure.0, "monotonic/wrong");
    assert!(failure.1.starts_with("ids:"), "{failure:?}");
}
