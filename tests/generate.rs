//! Time-based generation beyond the conformance corpus's encode cases and
//! generate refusals, which `tests/corpus.rs` runs: the random width, the
//! delimiter, the time bound and option, the defaults, the byte source,
//! size parsing, error text and the `std` and `getrandom` defaults.

use uxid::{ByteSource, Error, Options, Size, generate_with};

/// A byte source that must never be called: the request fixes or omits the
/// random field.
struct Unreachable;

impl ByteSource for Unreachable {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        panic!("the byte source was called");
    }
}

/// A byte source that writes a repeating byte and records each width asked for.
struct Repeating {
    byte: u8,
    asked: Vec<usize>,
}

impl ByteSource for Repeating {
    fn fill(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        self.asked.push(buf.len());
        buf.fill(self.byte);
        Ok(())
    }
}

/// A byte source that always fails with the error it holds.
struct Failing(Error);

impl ByteSource for Failing {
    fn fill(&mut self, _buf: &mut [u8]) -> Result<(), Error> {
        Err(self.0.clone())
    }
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

const T: u64 = 1_700_000_000_000;

// Sabotage: ignoring `rand_size` and keeping the size's width turns this red.
#[test]
fn rand_size_overrides_the_width_of_the_size() {
    let mut source = Repeating {
        byte: 0xff,
        asked: Vec::new(),
    };
    let options = Options::new().size(Size::S).rand_size(10).time(T);
    let id = generate_with(&options, &mut source, 0).unwrap();
    assert_eq!(id, "01hf7yat00zzzzzzzzzzzzzzzz");
    assert_eq!(source.asked, [10]);

    let options = Options::new().size(Size::Xl).rand_size(0).time(T);
    assert_eq!(
        generate_with(&options, &mut Unreachable, 0).unwrap(),
        "01hf7yat00"
    );
}

// Sabotage: hard-coding `_` in place of the delimiter option turns this red.
#[test]
fn the_delimiter_sits_between_prefix_and_body() {
    let rand = hex("0123");
    let options = Options::new()
        .prefix("usr")
        .delimiter("--")
        .size(Size::S)
        .time(T)
        .rand(&rand);
    assert_eq!(
        generate_with(&options, &mut Unreachable, 0).unwrap(),
        "usr--01hf7yat0004h1"
    );
}

// Sabotage: accepting an empty delimiter turns this red.
#[test]
fn an_empty_delimiter_is_refused() {
    let options = Options::new().prefix("usr").delimiter("").time(T);
    let error = generate_with(&options, &mut Unreachable, 0).unwrap_err();
    assert_eq!(error, Error::EmptyDelimiter);
    assert_eq!(error.code(), "empty_delimiter");
}

// Sabotage: dropping the 48-bit time bound turns this red.
#[test]
fn a_time_past_48_bits_is_refused() {
    let options = Options::new().size(Size::Xs).time(1 << 48);
    let error = generate_with(&options, &mut Unreachable, 0).unwrap_err();
    assert_eq!(error, Error::TimeOutOfRange);
    assert_eq!(error.code(), "time_out_of_range");

    // The clock reading is held to the same bound when no time is set.
    let options = Options::new().size(Size::Xs);
    assert_eq!(
        generate_with(&options, &mut Unreachable, 1 << 48).unwrap_err(),
        Error::TimeOutOfRange
    );
}

// Sabotage: preferring `now` over the time option turns this red.
#[test]
fn the_time_option_wins_over_now() {
    let options = Options::new().size(Size::Xs).time(T);
    assert_eq!(
        generate_with(&options, &mut Unreachable, 0).unwrap(),
        "01hf7yat00"
    );
    assert_eq!(
        generate_with(&options, &mut Unreachable, 1).unwrap(),
        "01hf7yat00"
    );

    let options = Options::new().size(Size::Xs);
    assert_eq!(
        generate_with(&options, &mut Unreachable, T).unwrap(),
        "01hf7yat00"
    );
}

// Sabotage: a default size other than xl turns this red.
#[test]
fn the_defaults_are_xl_standard_lower_and_no_prefix() {
    let mut source = Repeating {
        byte: 0xff,
        asked: Vec::new(),
    };
    let id = generate_with(&Options::default(), &mut source, T).unwrap();
    assert_eq!(id, "01hf7yat00zzzzzzzzzzzzzzzz");
    assert_eq!(source.asked, [10]);
}

// Sabotage: giving compact s the standard width (2 bytes) turns this red.
#[test]
fn the_byte_source_fills_exactly_the_random_width() {
    let mut source = Repeating {
        byte: 0x00,
        asked: Vec::new(),
    };
    for size in [Size::Xs, Size::S, Size::M, Size::L, Size::Xl] {
        for compact in [false, true] {
            let options = Options::new().size(size).compact_time(compact).time(T);
            generate_with(&options, &mut source, 0).unwrap();
        }
    }
    // xs standard has no random field and does not call the source.
    assert_eq!(source.asked, [1, 2, 3, 5, 6, 7, 8, 10, 10]);
}

// Sabotage: mapping the source's error to `Error::ByteSource` turns this red.
#[test]
fn a_byte_source_failure_is_returned_unchanged() {
    let options = Options::new().prefix("usr").time(T);
    for returned in [Error::ByteSource, Error::TimeOutOfRange] {
        let error = generate_with(&options, &mut Failing(returned.clone()), 0).unwrap_err();
        assert_eq!(error, returned);
    }
    assert_eq!(Error::ByteSource.code(), "byte_source");
}

// Sabotage: a `&mut T` impl that leaves the buffer untouched turns this red.
#[test]
fn a_mutable_reference_to_a_source_is_a_source() {
    let mut inner = Repeating {
        byte: 0xff,
        asked: Vec::new(),
    };
    let mut source = &mut inner;
    let options = Options::new().size(Size::S).time(T);
    let id = generate_with(&options, &mut source, 0).unwrap();
    assert_eq!(id, "01hf7yat00zzz1");
    assert_eq!(inner.asked, [2]);
}

// Sabotage: a `Box<T>` impl that leaves the buffer untouched turns this red.
#[test]
fn a_boxed_source_is_a_source() {
    let mut source: Box<dyn ByteSource> = Box::new(Repeating {
        byte: 0xff,
        asked: Vec::new(),
    });
    let options = Options::new().size(Size::S).time(T);
    let id = generate_with(&options, &mut source, 0).unwrap();
    assert_eq!(id, "01hf7yat00zzz1");
}

// Sabotage: removing a long spelling from `Size`'s `FromStr` turns this red.
#[test]
fn sizes_parse_from_short_and_long_spellings() {
    for (short, long, size) in [
        ("xs", "xsmall", Size::Xs),
        ("s", "small", Size::S),
        ("m", "medium", Size::M),
        ("l", "large", Size::L),
        ("xl", "xlarge", Size::Xl),
    ] {
        assert_eq!(short.parse::<Size>(), Ok(size));
        assert_eq!(long.parse::<Size>(), Ok(size));
    }
    assert_eq!("XL".parse::<Size>(), Err(Error::UnknownSize));
    assert_eq!("".parse::<Size>(), Err(Error::UnknownSize));
    assert_eq!(Error::UnknownSize.code(), "unknown_size");
}

// Sabotage: a `Display` that prints the code alone turns this red.
#[test]
fn errors_display_a_sentence_naming_the_refusal() {
    for error in [
        Error::CompactTimeOutOfRange,
        Error::RandWidthMismatch,
        Error::RandSizeTooLarge,
        Error::TimeOutOfRange,
        Error::EmptyDelimiter,
        Error::UnknownSize,
        Error::ByteSource,
    ] {
        let text = error.to_string();
        let tail = format!(" ({})", error.code());
        assert!(text.ends_with(&tail), "{text}");
        assert!(text.len() > tail.len(), "{text}");
        let source: &dyn core::error::Error = &error;
        assert!(source.source().is_none());
    }
}

#[cfg(all(feature = "std", feature = "getrandom"))]
mod defaults {
    use uxid::{ByteSource, GetrandomSource, Options, Size, generate, now_millis};

    /// How far the wall clock may step between two reads in a test (an NTP
    /// correction can move it backwards) before the test calls it a failure.
    const SLACK_MS: u64 = 10_000;

    /// The time field of a standard-time body, read back as milliseconds.
    fn standard_time(body: &str) -> u64 {
        body[..10].chars().fold(0, |time, c| {
            let value = "0123456789abcdefghjkmnpqrstvwxyz".find(c).unwrap() as u64;
            time * 32 + value
        })
    }

    // Sabotage: a `generate` that passes 0 in place of the system clock turns this red.
    #[test]
    fn generate_reads_the_clock_and_the_platform_source() {
        let before = now_millis().unwrap();
        let id = generate(&Options::new().prefix("usr").size(Size::Xs)).unwrap();
        let after = now_millis().unwrap();
        assert_eq!(id.len(), "usr_".len() + 10);
        assert!(id.starts_with("usr_"));
        let time = standard_time(&id[4..]);
        assert!(
            before.saturating_sub(SLACK_MS) <= time && time <= after + SLACK_MS,
            "{before} <= {time} <= {after}, give or take {SLACK_MS} ms"
        );

        let full = generate(&Options::new()).unwrap();
        assert_eq!(full.len(), 26);
        assert!(
            full.chars()
                .all(|c| "0123456789abcdefghjkmnpqrstvwxyz".contains(c))
        );
    }

    // Sabotage: a `now_millis` that answers 0 turns this red.
    #[test]
    fn now_millis_reads_the_system_clock() {
        let system = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let clock = now_millis().unwrap();
        assert!(
            clock.abs_diff(system) <= SLACK_MS,
            "{system} {clock}, give or take {SLACK_MS} ms"
        );
    }

    // Sabotage: a `generate` that draws from an all-zero source turns this red.
    #[test]
    fn generate_draws_its_random_field_from_the_platform_source() {
        let first = generate(&Options::new().time(0)).unwrap();
        let second = generate(&Options::new().time(0)).unwrap();
        // Two 80-bit random fields from a secure source are equal with
        // probability 2^-80, and all zeros with the same.
        assert_ne!(first[10..], second[10..]);
        assert_ne!(&first[10..], "0000000000000000");
    }

    // Sabotage: a `GetrandomSource` that leaves the buffer untouched turns this red.
    #[test]
    fn getrandom_source_fills_the_buffer() {
        let mut buf = [0u8; 16];
        GetrandomSource.fill(&mut buf).unwrap();
        assert_ne!(buf, [0u8; 16]);
    }
}
