//! Deterministic bodies: a SHA-256 hash of a caller's string, so the same
//! inputs always give the same id.

use alloc::string::String;

use sha2::{Digest, Sha256};

use crate::base32;
use crate::{Case, Size};

/// The number of hash characters a deterministic body carries at `size`:
/// its standard body length less the marker.
pub(crate) const fn hash_chars(size: Size) -> u8 {
    match size {
        Size::Xs => 9,
        Size::S => 13,
        Size::M => 17,
        Size::L => 21,
        Size::Xl => 25,
    }
}

/// Appends the deterministic body of `from` under `prefix` at `size`: the
/// marker, then the first `count x 5` bits of SHA-256 over `prefix`, a zero
/// byte, `from`, a zero byte and one byte holding `count`. Case is not part
/// of the hash input.
pub(crate) fn push_body(out: &mut String, prefix: &str, from: &str, size: Size, case: Case) {
    let count = hash_chars(size);
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update([0u8]);
    hasher.update(from.as_bytes());
    hasher.update([0u8, count]);
    let digest = hasher.finalize();
    out.push(match case {
        Case::Lower => 'z',
        Case::Upper => 'Z',
    });
    base32::push_leading_bits(out, &digest, u32::from(count), case);
}

#[cfg(test)]
mod tests {
    use super::{hash_chars, push_body};
    use crate::{Case, Size};
    use alloc::string::String;

    // Sabotage: giving `S` 14 hash characters (its body length) turns this red.
    #[test]
    fn hash_characters_are_the_standard_length_less_the_marker() {
        for (size, length) in [
            (Size::Xs, 10),
            (Size::S, 14),
            (Size::M, 18),
            (Size::L, 22),
            (Size::Xl, 26),
        ] {
            assert_eq!(usize::from(hash_chars(size)) + 1, length);
            let mut out = String::new();
            push_body(&mut out, "usr", "a", size, Case::Lower);
            assert_eq!(out.len(), length);
        }
    }
}
