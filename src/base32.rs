//! Crockford Base32 as the wire format writes it: 32 symbols in ASCII order,
//! `I`, `L`, `O` and `U` left out, no padding and no check symbol.

use alloc::string::String;

use crate::Case;

const LOWER: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
const UPPER: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn symbol(value: u8, case: Case) -> char {
    let table = match case {
        Case::Lower => LOWER,
        Case::Upper => UPPER,
    };
    char::from(table[usize::from(value & 31)])
}

/// Appends the low `count` 5-bit groups of `value`, most significant group
/// first. The first group holds whatever bits remain above the others, which
/// is how the standard 48-bit time writes its 3-bit first group.
pub(crate) fn push_integer(out: &mut String, value: u64, count: u32, case: Case) {
    for i in (0..count).rev() {
        let group = ((value >> (5 * i)) & 31) as u8;
        out.push(symbol(group, case));
    }
}

/// Appends bytes as 5-bit groups, most significant bit first. When the bit
/// count is not a multiple of 5 the last group is short, and it is written as
/// its raw value rather than shifted or padded: the two bytes `ff ff` are
/// `zzz1`.
pub(crate) fn push_bytes(out: &mut String, bytes: &[u8], case: Case) {
    let mut buffer: u16 = 0;
    let mut bits: u32 = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let group = ((buffer >> bits) & 31) as u8;
            out.push(symbol(group, case));
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(symbol(buffer as u8, case));
    }
}

/// Appends the first `count` 5-bit groups of `bytes`, most significant bit
/// first, with no short group: the caller asks for at most `8 x bytes.len()`
/// bits, and any bits after the last group are left out.
pub(crate) fn push_leading_bits(out: &mut String, bytes: &[u8], count: u32, case: Case) {
    let mut buffer: u16 = 0;
    let mut bits: u32 = 0;
    let mut written: u32 = 0;
    for &byte in bytes {
        if written == count {
            break;
        }
        buffer = (buffer << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 && written < count {
            bits -= 5;
            let group = ((buffer >> bits) & 31) as u8;
            out.push(symbol(group, case));
            written += 1;
        }
        buffer &= (1 << bits) - 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{push_bytes, push_integer, push_leading_bits};
    use crate::Case;
    use alloc::string::String;

    // Sabotage: swapping `j` and `k` in the lower-case table turns this red.
    #[test]
    fn every_value_writes_its_symbol_in_both_cases() {
        for (case, alphabet) in [
            (Case::Lower, "0123456789abcdefghjkmnpqrstvwxyz"),
            (Case::Upper, "0123456789ABCDEFGHJKMNPQRSTVWXYZ"),
        ] {
            let mut out = String::new();
            for value in 0..32 {
                push_integer(&mut out, value, 1, case);
            }
            assert_eq!(out, alphabet);
        }
    }

    // Sabotage: writing the short last group shifted left (padded) turns this red.
    #[test]
    fn the_short_last_group_is_its_raw_value() {
        let mut out = String::new();
        push_bytes(&mut out, &[0xff, 0xff], Case::Lower);
        assert_eq!(out, "zzz1");
    }

    // Sabotage: writing the bits after the last whole group as a short group turns this red.
    #[test]
    fn leading_bits_stop_at_the_count_with_no_short_group() {
        // 0b00001_00010_00011_0 ...: three whole groups, then the rest is left out.
        let bytes = [0b0000_1000, 0b1000_0110, 0xff];
        let mut out = String::new();
        push_leading_bits(&mut out, &bytes, 3, Case::Lower);
        assert_eq!(out, "123");
        let mut upper = String::new();
        push_leading_bits(&mut upper, &[0xff; 4], 6, Case::Upper);
        assert_eq!(upper, "ZZZZZZ");
    }
}
