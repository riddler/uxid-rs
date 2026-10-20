### Added

- `generate_with(&Options, &mut impl ByteSource, now)` builds a time-based id as uxid-ex's wire format defines it, from the options prefix (an empty string is no prefix), size (`Size`, parsed from `xs`, `s`, `m`, `l`, `xl` and the long spellings), rand_size, compact_time, case, delimiter and time, taking random bytes from the byte source and the time from `now` when the options set none. It needs no feature and builds under `no_std`.
- `generate(&Options)`, behind the default `std` and `getrandom` features together, is `generate_with` with the platform's secure source and the system clock.
- `now_millis()`, behind the `std` feature, reads the system clock in milliseconds since the Unix epoch, for passing as `now` beside a byte source of your own.
- The `ByteSource` trait fills a byte slice and returns a `Result`; a `&mut` reference to a source and a `Box` of one are sources too. `GetrandomSource`, behind the `getrandom` feature, is the platform's secure source.
- `Options::rand` fixes the random field's bytes for reproducible test vectors only; it is never a source of randomness.
- `Error`, a `#[non_exhaustive]` enum whose `code()` is the refusal's stable name: `compact_time_out_of_range`, `rand_width_mismatch` and `rand_size_too_large` as the wire format names them, and this crate's `time_out_of_range`, `empty_delimiter`, `unknown_size` and `byte_source`. `Error` and `Options` are `Clone`, not `Copy`.
