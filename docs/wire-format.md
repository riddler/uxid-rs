# Wire format

This crate does not restate the format. The normative layout of a uxid - the
alphabet, the bit packing, the sizes, the prefix grammar, the time and
deterministic bodies, monotonic generation and decoding - is the reference
implementation's guide, read at the release tag this crate's vendored corpus
names:

[UXID wire format, at v2.10.0](https://github.com/riddler/uxid-ex/blob/v2.10.0/guides/wire-format.md)

The tag is the one recorded in `conformance/SOURCE.json`, beside the commit it
names and the hash of the corpus. A change that re-vendors the corpus at
another tag updates this pointer and the README's links to the reference in
the same change; no test checks them.

## The corpus is the arbiter

The conformance corpus is vendored under `conformance/`, byte for byte from
the same tag. The test suite runs every case in it with no skip list
(`tests/corpus.rs`) and checks the vendored files against the recorded hash
(`tests/corpus_hash.rs`). This crate is held to the corpus: a case it cannot
pass is a bug here or a question for the reference implementation, never a
skip, and a difference between the guide and the corpus is raised there
rather than worked round here. An answer the corpus does not fix follows the
guide at the tag.

## Decoding a compact time

A compact body carries only the low 40 bits of its time, and a decoder
supplies the high bits from its own clock (the guide's "Decoding the time").
`decode` reads the system clock; `decode_with` takes the clock as an argument
in milliseconds. The corpus's compact decode cases hold for a decoder clock
from 1699913600000 ms inclusive to 2164577117183 ms exclusive, as the
corpus's own README (`conformance/README.md`, "Compact times and the
decoder's clock") sets out. The test suite injects the clock rather than
reading the system time: it decodes each such case at the window's first
millisecond, at 1700000000000 ms and at the window's last millisecond. A
standard body carries its whole time and reads no clock.

## The validity checks

The guide does not specify a validity check. The reference's `valid?` and
`deterministic?` take a delimiter option, and `valid?` a prefix to require;
this crate takes the same inputs. `is_valid(id)` and `is_deterministic(id)`
check under the delimiter `_`; `is_valid_with(id, delimiter, prefix)` and
`is_deterministic_with(id, delimiter)` take the delimiter, the first with an
`Option<&str>` prefix the id must carry exactly (`None` or an empty prefix
accepts any prefix or none). Each answers whether `decode_with` decodes the
id under that delimiter, and the deterministic checks also whether its body
starts with the marker `z` or `Z`, so they accept exactly what the decoder
accepts. The reference's own checks are looser: its `valid?` tests the
body's alphabet and a minimum length rather than the length table, and its
`deterministic?` reads only the first body character. The checks here read
no clock. The test suite runs every corpus decode case and every decode
refusal through them, under `_` and under a second delimiter.

## What this crate decides for itself

The Rust API: the function, type and method names, the `Options` builder, the
`ByteSource` trait, the features, and the `Error` variants and their `code()`
names where the guide names no refusal. Those are documented in the API
reference on [docs.rs](https://docs.rs/uxid), built from `src/`.
