### Added

- `Options::from` asks for a deterministic id: the marker `z` (`Z` in upper case) followed by the first characters of a SHA-256 digest of the prefix, a zero byte, `from` as UTF-8, a zero byte and one byte holding the number of hash characters, at the size's standard body length, as uxid-ex's wire format defines it. `generate_with` and `generate` build it; with `from` the byte source is never called and the time, `now`, `compact_time` and `rand_size` are not read. It needs no feature and builds under `no_std`.
- `Error::RandWithFrom` (code `rand_with_from`, a corpus name) refuses fixed random bytes asked for with `from`.
