//! Monotonic generation against every case of the conformance corpus's
//! `conformance/monotonic.json` and its two monotonic generate refusals in
//! `conformance/invalid.json` (uxid-ex `v2.10.0`), each named by its case id,
//! plus the rule's edges the corpus does not reach. Every case file runs in
//! full once the corpus is vendored.

use uxid::{ByteSource, Case, Error, Monotonic, Options, Size, generate_with};

/// One recorded draw: a fresh value for the field, or a step.
#[derive(Debug, Clone, Copy)]
enum Draw {
    Seed(u128),
    Step(u128),
}

/// A byte source that replays recorded draws in order, as the bytes the
/// generator documents reading for each: a seed is `w` bytes big-endian, a
/// step is `step - 1` in `ceil(w / 2)` bytes big-endian. It checks that each
/// request has the length its draw needs, and that every draw is used.
struct Replay {
    width: usize,
    draws: Vec<Draw>,
    next: usize,
}

impl Replay {
    fn new(width: usize, draws: &[Draw]) -> Self {
        Self {
            width,
            draws: draws.to_vec(),
            next: 0,
        }
    }

    fn finished(&self) -> bool {
        self.next == self.draws.len()
    }
}

impl ByteSource for Replay {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        let draw = self.draws[self.next];
        self.next += 1;
        let (value, count) = match draw {
            Draw::Seed(value) => (value, self.width),
            Draw::Step(step) => (step - 1, self.width.div_ceil(2)),
        };
        assert_eq!(
            buf.len(),
            count,
            "draw {draw:?} read as {} bytes",
            buf.len()
        );
        buf.copy_from_slice(&value.to_be_bytes()[16 - count..]);
        Ok(())
    }
}

/// A byte source that writes the bytes it holds, in order, one request at a
/// time.
struct Bytes(Vec<Vec<u8>>);

impl ByteSource for Bytes {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        buf.copy_from_slice(&self.0.remove(0));
        Ok(())
    }
}

/// A byte source that always fails.
struct Failing;

impl ByteSource for Failing {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        Err(Error::ByteSource)
    }
}

/// A byte source that must never be called.
struct Unreachable;

impl ByteSource for Unreachable {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        panic!("the byte source was called");
    }
}

struct Vector {
    id: &'static str,
    prefix: Option<&'static str>,
    size: Size,
    compact: bool,
    width: usize,
    draws: &'static [Draw],
    times: &'static [u64],
    expected: &'static [&'static str],
}

const T: u64 = 1_700_000_000_000;

/// The corpus's monotonic cases. Each draw list is the case's
/// `initial_seed`, then its `steps` in order, with its `overflow_seed`
/// after the step that overflows the field.
const CASES: &[Vector] = &[
    Vector {
        id: "monotonic/xl-same-millisecond",
        prefix: Some("usr"),
        size: Size::Xl,
        compact: false,
        width: 10,
        draws: &[
            Draw::Seed(123_456_789),
            Draw::Step(1),
            Draw::Step(1_099_511_627_776),
            Draw::Step(42),
        ],
        times: &[T, T, T, T],
        expected: &[
            "usr_01hf7yat0000000000003nqk8n",
            "usr_01hf7yat0000000000003nqk8p",
            "usr_01hf7yat0000000001003nqk8p",
            "usr_01hf7yat0000000001003nqka0",
        ],
    },
    Vector {
        id: "monotonic/m-clock-moves-back",
        prefix: Some("usr"),
        size: Size::M,
        compact: false,
        width: 5,
        draws: &[
            Draw::Seed(1000),
            Draw::Step(5),
            Draw::Step(1_048_576),
            Draw::Step(7),
        ],
        times: &[T, T - 1, T - 10, T],
        expected: &[
            "usr_01hf7yat00000000z8",
            "usr_01hf7yat00000000zd",
            "usr_01hf7yat00000100zd",
            "usr_01hf7yat00000100zm",
        ],
    },
    Vector {
        id: "monotonic/s-overflow-bumps-time",
        prefix: Some("usr"),
        size: Size::S,
        compact: false,
        width: 2,
        draws: &[
            Draw::Seed(65000),
            Draw::Step(200),
            Draw::Step(250),
            Draw::Step(100),
            Draw::Seed(1234),
            Draw::Step(7),
            Draw::Step(3),
        ],
        times: &[T, T, T, T, T, T + 1],
        expected: &[
            "usr_01hf7yat00zqm0",
            "usr_01hf7yat00ztr0",
            "usr_01hf7yat00zyn0",
            "usr_01hf7yat010k90",
            "usr_01hf7yat010kc1",
            "usr_01hf7yat010ke0",
        ],
    },
    Vector {
        id: "monotonic/xs-compact-overflow-clock-back",
        prefix: Some("usr"),
        size: Size::Xs,
        compact: true,
        width: 1,
        draws: &[
            Draw::Seed(250),
            Draw::Step(5),
            Draw::Step(16),
            Draw::Seed(9),
        ],
        times: &[T, T, T - 1],
        expected: &["usr_hf7yat00z2", "usr_hf7yat00z7", "usr_hf7yat0111"],
    },
    Vector {
        id: "monotonic/l-compact-single",
        prefix: Some("usr"),
        size: Size::L,
        compact: true,
        width: 8,
        draws: &[Draw::Seed(0)],
        times: &[T],
        expected: &["usr_hf7yat000000000000000"],
    },
    Vector {
        id: "monotonic/s-compact-no-prefix-largest-step",
        prefix: None,
        size: Size::S,
        compact: true,
        width: 3,
        draws: &[Draw::Seed(0), Draw::Step(4096)],
        times: &[T, T],
        expected: &["hf7yat0000000", "hf7yat0000800"],
    },
];

// Sabotage: carrying `now` in place of the last time on a step turns m-clock-moves-back red.
#[test]
fn monotonic_cases_match_the_corpus() {
    for case in CASES {
        let options = Options::new()
            .prefix(case.prefix.unwrap_or(""))
            .size(case.size)
            .compact_time(case.compact);
        let mut generator = Monotonic::new(&options).unwrap();
        let mut source = Replay::new(case.width, case.draws);
        let ids: Vec<String> = case
            .times
            .iter()
            .map(|&now| generator.generate_with(&mut source, now).unwrap())
            .collect();
        assert_eq!(ids, case.expected, "{}", case.id);
        assert!(source.finished(), "{}: draws left unread", case.id);
    }
}

// Sabotage: building a generator with `from` set turns this red.
#[test]
fn invalid_generate_from_with_monotonic_is_refused() {
    // invalid/generate-from-with-monotonic
    let options = Options::new().prefix("usr").from("alice@example.com");
    let error = Monotonic::new(&options).err();
    assert_eq!(error, Some(Error::FromWithMonotonic));
    assert_eq!(Error::FromWithMonotonic.code(), "from_with_monotonic");
    // `from` is refused first, before fixed `rand` bytes.
    let both = options.rand(&[0x01]);
    assert_eq!(Monotonic::new(&both).err(), Some(Error::FromWithMonotonic));
}

// Sabotage: building a generator with fixed `rand` bytes set turns this red.
#[test]
fn invalid_generate_rand_with_monotonic_is_refused() {
    // invalid/generate-rand-with-monotonic
    let options = Options::new()
        .prefix("usr")
        .rand(&[0x01, 0x02])
        .size(Size::S)
        .time(T);
    let error = Monotonic::new(&options).err();
    assert_eq!(error, Some(Error::RandWithMonotonic));
    assert_eq!(Error::RandWithMonotonic.code(), "rand_with_monotonic");
}

// Sabotage: dropping the zero-width check turns this red.
#[test]
fn a_generator_with_no_random_field_is_refused() {
    for options in [
        Options::new().size(Size::Xs),
        Options::new().size(Size::M).rand_size(0),
    ] {
        let error = Monotonic::new(&options).err();
        assert_eq!(error, Some(Error::EmptyRandWithMonotonic));
    }
    assert_eq!(
        Error::EmptyRandWithMonotonic.code(),
        "empty_rand_with_monotonic"
    );
    let mut xs = Monotonic::new(&Options::new().size(Size::Xs).rand_size(1)).unwrap();
    let id = xs.generate_with(&mut Bytes(vec![vec![0x00]]), T).unwrap();
    assert_eq!(id, "01hf7yat0000");
}

// Sabotage: moving the random width cap to 9 or to 11 bytes turns this red.
#[test]
fn a_random_width_above_ten_bytes_is_refused() {
    let error = Monotonic::new(&Options::new().rand_size(11)).err();
    assert_eq!(error, Some(Error::RandSizeTooLarge));
    assert!(Monotonic::new(&Options::new().rand_size(10)).is_ok());
}

// Sabotage: accepting an empty delimiter turns this red.
#[test]
fn an_empty_delimiter_is_refused() {
    let error = Monotonic::new(&Options::new().prefix("usr").delimiter("")).err();
    assert_eq!(error, Some(Error::EmptyDelimiter));
}

// Sabotage: moving the time option's bound one millisecond either way turns this red.
#[test]
fn a_time_option_past_48_bits_is_refused() {
    let error = Monotonic::new(&Options::new().time(1 << 48)).err();
    assert_eq!(error, Some(Error::TimeOutOfRange));
    assert!(Monotonic::new(&Options::new().time((1 << 48) - 1)).is_ok());
}

// Sabotage: dropping the 48-bit bound on `now` turns this red.
#[test]
fn a_now_past_48_bits_is_refused() {
    let mut generator = Monotonic::new(&Options::new()).unwrap();
    let mut source = Bytes(vec![vec![0; 10]]);
    let error = generator.generate_with(&mut source, 1 << 48).err();
    assert_eq!(error, Some(Error::TimeOutOfRange));
    let latest = generator
        .generate_with(&mut Bytes(vec![vec![0; 10]]), (1 << 48) - 1)
        .unwrap();
    assert_eq!(latest, "7zzzzzzzzz0000000000000000");
}

// Sabotage: moving to `T + 1` without the 48-bit bound turns this red.
#[test]
fn an_overflow_in_the_last_millisecond_is_refused_and_keeps_the_state() {
    let last = (1 << 48) - 1;
    let mut generator = Monotonic::new(&Options::new().size(Size::S)).unwrap();
    let first = generator
        .generate_with(&mut Replay::new(2, &[Draw::Seed(0xffff)]), last)
        .unwrap();
    assert_eq!(first, "7zzzzzzzzzzzz1");
    let mut overflow = Replay::new(2, &[Draw::Step(1), Draw::Seed(0)]);
    assert_eq!(
        generator.generate_with(&mut overflow, last),
        Err(Error::TimeOutOfRange)
    );
    // The refusal drew the step and no fresh value.
    assert_eq!(overflow.next, 1);
    // The refused call left the state as it was: the field still overflows.
    assert_eq!(
        generator.generate_with(&mut Replay::new(2, &[Draw::Step(1)]), last),
        Err(Error::TimeOutOfRange)
    );
}

// Sabotage: clearing the state before the draw turns this red.
#[test]
fn a_failing_source_leaves_the_state_as_it_was() {
    let mut generator = Monotonic::new(&Options::new().size(Size::S)).unwrap();
    assert_eq!(
        generator.generate_with(&mut Failing, T),
        Err(Error::ByteSource)
    );
    let first = generator
        .generate_with(&mut Replay::new(2, &[Draw::Seed(0x0100)]), T)
        .unwrap();
    assert_eq!(
        generator.generate_with(&mut Failing, T),
        Err(Error::ByteSource)
    );
    let second = generator
        .generate_with(&mut Replay::new(2, &[Draw::Step(1)]), T)
        .unwrap();
    assert_eq!(first, "01hf7yat000400");
    assert_eq!(second, "01hf7yat000401");
}

// Sabotage: skipping the compact guard, or applying it to standard time, turns this red.
#[test]
fn a_compact_time_at_the_marker_is_refused() {
    let marker = 31_u64 << 35;
    let options = Options::new().size(Size::S).compact_time(true);
    let mut generator = Monotonic::new(&options).unwrap();
    let mut source = Replay::new(3, &[Draw::Seed(0)]);
    let error = generator.generate_with(&mut source, marker).err();
    assert_eq!(error, Some(Error::CompactTimeOutOfRange));

    // Standard time has no marker guard.
    let mut standard = Monotonic::new(&Options::new().size(Size::S)).unwrap();
    let mut source = Replay::new(2, &[Draw::Seed(0)]);
    assert!(standard.generate_with(&mut source, marker).is_ok());

    // An overflow onto the marker is refused too.
    let mut generator = Monotonic::new(&options).unwrap();
    generator
        .generate_with(&mut Replay::new(3, &[Draw::Seed(0xff_ffff)]), marker - 1)
        .unwrap();
    let mut overflow = Replay::new(3, &[Draw::Step(1), Draw::Seed(0)]);
    assert_eq!(
        generator.generate_with(&mut overflow, marker - 1),
        Err(Error::CompactTimeOutOfRange)
    );
}

// Sabotage: reducing the step's bytes modulo 2^(4w) - 1 instead of masking 4w bits turns this red.
#[test]
fn a_one_byte_step_takes_every_value_from_1_to_16_equally_often() {
    let options = Options::new().size(Size::S).rand_size(1).time(T);
    let mut counts = [0_u32; 17];
    for byte in 0..=255_u8 {
        let mut generator = Monotonic::new(&options).unwrap();
        let mut source = Bytes(vec![vec![0x00], vec![byte]]);
        generator.generate_with(&mut source, 0).unwrap();
        let id = generator.generate_with(&mut source, 0).unwrap();
        let step = (1..=16_u8)
            .find(|&step| {
                generate_with(&options.clone().rand(&[step]), &mut Unreachable, 0).unwrap() == id
            })
            .unwrap();
        assert_eq!(step, (byte & 0x0f) + 1, "byte {byte:#04x}");
        counts[usize::from(step)] += 1;
    }
    assert_eq!(counts[0], 0);
    assert!(counts[1..].iter().all(|&count| count == 16), "{counts:?}");
}

// Sabotage: keeping all 16 bits of the step's two bytes turns this red.
#[test]
fn a_step_keeps_only_the_low_half_width_bits() {
    let options = Options::new().rand_size(3).time(T);
    for (bytes, expected) in [
        ([0xff, 0xff], [0x00, 0x10, 0x00]),
        ([0xf0, 0x00], [0x00, 0x00, 0x01]),
        ([0x0a, 0xbc], [0x00, 0x0a, 0xbd]),
    ] {
        let mut generator = Monotonic::new(&options).unwrap();
        let mut source = Bytes(vec![vec![0, 0, 0], bytes.to_vec()]);
        generator.generate_with(&mut source, 0).unwrap();
        let id = generator.generate_with(&mut source, 0).unwrap();
        let want = generate_with(&options.clone().rand(&expected), &mut Unreachable, 0).unwrap();
        assert_eq!(id, want, "step bytes {bytes:02x?}");
    }
}

// Sabotage: masking the ten-byte field's step to fewer than 40 bits turns this red.
#[test]
fn a_ten_byte_step_reaches_two_to_the_fortieth() {
    let mut generator = Monotonic::new(&Options::new()).unwrap();
    let mut source = Bytes(vec![vec![0; 10], vec![0xff; 5]]);
    generator.generate_with(&mut source, T).unwrap();
    let id = generator.generate_with(&mut source, T).unwrap();
    assert_eq!(id, "01hf7yat000000000100000000");
}

// Sabotage: carrying `now` in place of the time option turns this red.
#[test]
fn the_time_option_is_the_time_of_every_call() {
    let options = Options::new().prefix("cus").size(Size::S).time(T);
    let mut generator = Monotonic::new(&options).unwrap();
    let mut source = Replay::new(2, &[Draw::Seed(0x0123), Draw::Step(2)]);
    let first = generator.generate_with(&mut source, 5).unwrap();
    let second = generator.generate_with(&mut source, T + 99).unwrap();
    assert_eq!(first, "cus_01hf7yat0004h1");
    assert_eq!(second, "cus_01hf7yat0004j1");
}

// Sabotage: stepping instead of seeding when the clock moves forward turns this red.
#[test]
fn a_later_millisecond_takes_a_fresh_seed_and_the_new_time() {
    let mut generator = Monotonic::new(&Options::new().prefix("cus").size(Size::S)).unwrap();
    let mut source = Replay::new(2, &[Draw::Seed(0xffff), Draw::Seed(0x0123)]);
    let first = generator.generate_with(&mut source, T).unwrap();
    let second = generator.generate_with(&mut source, T + 1).unwrap();
    assert_eq!(first, "cus_01hf7yat00zzz1");
    assert_eq!(second, "cus_01hf7yat0104h1");
    assert!(first < second);
}

// Sabotage: ignoring the case option turns this red.
#[test]
fn case_and_delimiter_apply_and_the_prefix_keeps_its_case() {
    let options = Options::new()
        .prefix("Usr")
        .size(Size::S)
        .case(Case::Upper)
        .delimiter("--");
    let mut generator = Monotonic::new(&options).unwrap();
    let id = generator
        .generate_with(&mut Replay::new(2, &[Draw::Seed(0xffff)]), T)
        .unwrap();
    assert_eq!(id, "Usr--01HF7YAT00ZZZ1");
}

// Sabotage: writing the delimiter for an empty prefix turns this red.
#[test]
fn an_empty_prefix_is_no_prefix() {
    let mut generator = Monotonic::new(&Options::new().size(Size::S)).unwrap();
    let id = generator
        .generate_with(&mut Replay::new(2, &[Draw::Seed(0x0123)]), T)
        .unwrap();
    assert_eq!(id, "01hf7yat0004h1");
}

#[cfg(all(feature = "std", feature = "getrandom"))]
// Sabotage: seeding afresh on every call turns this red.
#[test]
fn generate_increases_strictly_on_the_system_clock() {
    let mut generator = Monotonic::new(&Options::new().prefix("usr").size(Size::S)).unwrap();
    let ids: Vec<String> = (0..2000).map(|_| generator.generate().unwrap()).collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(ids.iter().all(|id| uxid::decode(id).is_ok()));
}

#[cfg(all(feature = "std", feature = "getrandom"))]
// Sabotage: carrying `now` in place of the time option in `generate_with` turns this red.
#[test]
fn generate_reads_the_time_option_before_the_clock() {
    let options = Options::new().prefix("usr").size(Size::S).time(T);
    let mut generator = Monotonic::new(&options).unwrap();
    let id = generator.generate().unwrap();
    assert!(id.starts_with("usr_01hf7yat00"), "{id}");
}

// Sabotage: an empty Display sentence for a monotonic error turns this red.
#[test]
fn the_monotonic_errors_name_their_codes() {
    for error in [
        Error::FromWithMonotonic,
        Error::RandWithMonotonic,
        Error::EmptyRandWithMonotonic,
    ] {
        let text = error.to_string();
        assert!(text.ends_with(&format!("({})", error.code())), "{text}");
        assert!(text.len() > error.code().len() + 3, "{text}");
    }
}

// Sabotage: an `Rc` field in the generator makes this fail to compile.
#[test]
fn the_generator_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Monotonic>();
}
