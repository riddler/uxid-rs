### Added

- `decode_with(id, delimiter, now)` reads an id back to a `Decoded` as uxid-ex's wire format defines it: the prefix (everything before the last delimiter found reading left to right, `None` with no delimiter), the size and time form the body length in bytes gives, whether the body is deterministic, and the time (`None` for a deterministic body; a compact time's high bits come from the clock `now`). The lengths 10 and 18 are always read as standard time. It needs no feature and builds under `no_std`, and it never panics on malformed input.
- `decode(id)`, behind the default `std` feature, is `decode_with` with the delimiter `_` and the system clock, which it reads only for a compact body.
- `Decoded`, read through `prefix()`, `size()`, `is_compact()`, `is_deterministic()` and `time()`; its fields are private.
- `is_valid(id)` and `is_deterministic(id)` answer whether a string decodes under `_`, and whether it decodes to a deterministic body. Neither reads a clock or needs a feature.
- `Error::Empty`, `Error::InvalidLength` and `Error::InvalidCharacter` (codes `empty`, `invalid_length` and `invalid_character`, the corpus names) refuse a malformed id, checked in that order; `Error::EmptyDelimiter` refuses an empty delimiter on decode too.
