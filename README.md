# UXID

[![MIT License][badge_license_url]](LICENSE)
[![CI][badge_ci_url]](https://github.com/riddler/uxid-rs/actions/workflows/ci.yml)

User eXperience focused IDentifiers for Rust: prefixed, K-sortable,
Stripe-style ids a person can read, copy and route back to their resource. An
id looks like `usr_01hf7yat0004h1`: the prefix names the resource, and the
body carries a time and randomness in lower-case Crockford Base32. The ids are
wire-compatible with the Elixir reference implementation,
[uxid-ex](https://github.com/riddler/uxid-ex), and the crate is checked
against that implementation's conformance corpus.

## Why UXID

An auto-increment key leaks how many rows a table holds and invites
enumeration; a random UUID fixes both but is long, unordered and anonymous, so
in a log line, a URL or a support thread nobody can tell what it points at, and
a double-click selects only part of it. A UXID names its resource in its
prefix, selects whole on a double-click, reads aloud without ambiguity, sorts
by creation time so it indexes well, and is generated in the application with
no coordination between nodes. Its size is tunable for low-cardinality
resources, a monotonic generator keeps a burst within one millisecond unique
and in order, and a deterministic mode maps the same input to the same id.

Because the format is shared, an id this crate makes is read by the other
implementations of the format, and the other way round.

## Install

Add `uxid` to your `Cargo.toml`:

```toml
[dependencies]
uxid = "1"
```

The 0.0.1 release of `uxid` on crates.io was a placeholder with no library
API; nothing carries over from it.

## Basic usage

```rust
use uxid::{Monotonic, Options, Size};

// A prefix names the resource; the body is a time and randomness.
let id = uxid::generate(&Options::new().prefix("usr"))?;
assert!(id.starts_with("usr_"));

// Shrink the random part for low-cardinality resources.
// Sizes: Xs, S, M, L and Xl (the default).
let short = uxid::generate(&Options::new().prefix("cus").size(Size::S))?;
assert_eq!(short.len(), "cus_".len() + 14);

// Deterministic: the same prefix and input give the same id, every time.
let named = uxid::generate(&Options::new().prefix("usr").from("alice@example.com"))?;
assert_eq!(named, "usr_zcvt7epac0t1ebcsjfyf7cwz25");
assert!(uxid::is_deterministic(&named));

// Read an id back. A malformed id is an `Err`, never a panic.
let decoded = uxid::decode("cus_01hf7yat0004h1")?;
assert_eq!(decoded.prefix(), Some("cus"));
assert_eq!(decoded.size(), Size::S);
assert_eq!(decoded.time(), Some(1_700_000_000_000));
assert!(uxid::is_valid("cus_01hf7yat0004h1"));
assert_eq!(uxid::decode("cus_"), Err(uxid::Error::Empty));

// Monotonic: ids from one generator increase strictly, also within one
// millisecond. You hold the generator; the crate keeps no global state.
let mut ids = Monotonic::new(&Options::new().prefix("usr").size(Size::M))?;
let first = ids.generate()?;
let second = ids.generate()?;
assert!(first < second);
# Ok::<(), uxid::Error>(())
```

A request the wire format refuses, such as a random width above 10 bytes,
returns a `uxid::Error`; match on the variant, or log its `code()`, a stable
`snake_case` name, rather than its `Display` text.

`generate`, `decode` and `Monotonic::generate` read the system clock when
they need a time, and the two that generate read the platform's random
source when they need random bytes. Each has a `_with`
form that takes them as arguments instead: `generate_with` takes a
`ByteSource` and the time, `decode_with` the delimiter and the time, and
`Monotonic::generate_with` a `ByteSource` and the time. Those forms need no
feature and build under `no_std`. Fixed random bytes (`Options::rand`) exist
for test vectors: an id built from them is exactly as guessable as those
bytes.

## Compatibility

- **Rust 1.85 or later** (`rust-version` in `Cargo.toml`). CI runs the test
  suite with every feature on Rust 1.85, and the full quality gate on the
  toolchain `mise.toml` pins.
- **`no_std` with `alloc`.** The crate is `#![no_std]` and needs an
  allocator; with the `std` feature off it uses `core` and `alloc` alone. CI
  builds it with no default features for a target with no standard library
  (`thumbv7em-none-eabihf`). The crate forbids `unsafe` code.
- **Dependencies.** `sha2`, always, for deterministic ids; `getrandom` and
  `serde`, each only with its feature.
- **Features:**

  | Feature | Default | What it adds |
  |---|---|---|
  | `std` | yes | `now_millis`, the system clock, and `decode`, which is `decode_with` with the delimiter `_` and that clock |
  | `getrandom` | yes | `GetrandomSource`, the platform's secure byte source |
  | `std` and `getrandom` together | yes | `generate`, which is `generate_with` with `GetrandomSource` and `now_millis`, and `Monotonic::generate`, which is `Monotonic::generate_with` with the same two |
  | `serde` | no | Declared for later use; it derives nothing yet |

  For `no_std`, turn the defaults off and keep `getrandom` if your target
  supports it:

  ```toml
  [dependencies]
  uxid = { version = "1", default-features = false, features = ["getrandom"] }
  ```

  With no feature at all, `generate_with`, `decode_with`, `is_valid`,
  `is_deterministic`, `Options`, `Decoded`, `Monotonic` with its
  `generate_with`, `ByteSource` and `Error` are all present, and you supply
  the byte source and the time.

## Documentation

- Look up
  - [The API reference on docs.rs](https://docs.rs/uxid): every public
    type and trait, every function and every inherent method, each with an
    example.
  - [The wire format](https://github.com/riddler/uxid-rs/blob/main/docs/wire-format.md):
    where the normative layout of an id lives, and which version of it this
    crate follows.
  - [The changelog](https://github.com/riddler/uxid-rs/blob/main/CHANGELOG.md):
    what changed in each version.
  - [The conformance corpus](https://github.com/riddler/uxid-rs/tree/main/conformance):
    the cases every implementation of the wire format passes, vendored here
    from uxid-ex.
- Understand
  - [The reference spec, at v2.10.0][guide_wire_format_url]: the normative
    layout of an id - alphabet, bit packing, sizes, prefix grammar,
    deterministic and monotonic bodies, and decoding.
  - [Why a UXID has a prefix, a time and randomness][guide_why_url]: what each
    part of an id is for, and what you trade when you tune it.
  - [Why monotonic mode exists and what it costs][guide_monotonic_url]:
    same-millisecond uniqueness and ordering, and the security tradeoff.
  - [_Designing APIs for humans: object IDs_][stripe_ids_url]: the Stripe id
    design many of UXID's choices follow.

## Development

After checking out the repository, run `mise install` for the Rust toolchain
and the cargo tools `mise.toml` pins. Then `mise run gate:loop` runs the inner
loop (format, lints, tests), and `mise run gate` runs the full quality gate:
the inner loop, the docs with every feature and with none, line coverage
through `cargo llvm-cov` and `cargo deny check`. The tests include the
conformance corpus, every case with no skip list.

A release is published by CI, never from a checkout: pushing a tag `vX.Y.Z`
that names the version in `Cargo.toml` runs `.github/workflows/release.yml`,
which checks the tag and runs the full gate, then publishes the crate to
[crates.io](https://crates.io).

## Contributing

Bug reports and pull requests are welcome on GitHub at
https://github.com/riddler/uxid-rs.

## License

The crate is available as open source under the terms of the
[MIT License](https://opensource.org/licenses/MIT).

<!-- LINKS -->
[guide_wire_format_url]: https://github.com/riddler/uxid-ex/blob/v2.10.0/guides/wire-format.md
[guide_why_url]: https://github.com/riddler/uxid-ex/blob/v2.10.0/guides/why-prefix-time-randomness.md
[guide_monotonic_url]: https://github.com/riddler/uxid-ex/blob/v2.10.0/guides/monotonic.md
[stripe_ids_url]: https://dev.to/stripe/designing-apis-for-humans-object-ids-3o5a

<!-- BADGES -->
[badge_license_url]: https://img.shields.io/badge/license-MIT-brightgreen.svg
[badge_ci_url]: https://github.com/riddler/uxid-rs/actions/workflows/ci.yml/badge.svg
