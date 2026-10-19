### Added

- `generate_with(&Options, &mut impl ByteSource, now)` builds a time-based id as uxid-ex's wire format defines it, from the options prefix (an empty string is no prefix), size (`Size`, parsed from `xs`, `s`, `m`, `l`, `xl` and the long spellings), rand_size, compact_time, case, delimiter and time, taking random bytes from the byte source and the time from `now` when the options set none. It needs no feature and builds under `no_std`.
- `generate(&Options)`, behind the default `std` and `getrandom` features, is `generate_with` with the platform's secure source and the system clock.
- The `ByteSource` trait fills a byte slice and returns a `Result`; `GetrandomSource`, behind the `getrandom` feature, is the platform's secure source.
- `Options::rand` fixes the random field's bytes for reproducible test vectors only; it is never a source of randomness.
- `Error`, a `#[non_exhaustive]` enum whose `code()` is the refusal's stable name: `compact_time_out_of_range`, `rand_width_mismatch` and `rand_size_too_large` as the wire format names them, and this crate's `time_out_of_range`, `empty_delimiter`, `unknown_size` and `byte_source`.
