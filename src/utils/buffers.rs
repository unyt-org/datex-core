use core::fmt::Write;
use itertools::Itertools;
use crate::stdlib::string::String;
use crate::stdlib::string::ToString;
use crate::stdlib::vec::Vec;
use core::convert::TryInto;
use core::iter::FromIterator;

/*
read functions for primitive data types on a u8 array, also increments the index
 */

pub fn read_u8(buffer: &[u8], index: &mut usize) -> u8 {
    let val = buffer[*index];
    *index += 1;
    val
}

pub fn read_i8(buffer: &[u8], index: &mut usize) -> i8 {
    let slice = &buffer[*index..*index + 1];
    *index += 1;
    i8::from_le_bytes(slice.try_into().unwrap())
}

pub fn read_u16(buffer: &[u8], index: &mut usize) -> u16 {
    let slice = &buffer[*index..*index + 2];
    *index += 2;
    u16::from_be_bytes(slice.try_into().unwrap())
}
pub fn read_i16(buffer: &[u8], index: &mut usize) -> i16 {
    let slice = &buffer[*index..*index + 2];
    *index += 2;
    i16::from_be_bytes(slice.try_into().unwrap())
}

pub fn read_i32(buffer: &[u8], index: &mut usize) -> i32 {
    let slice = &buffer[*index..*index + 4];
    *index += 4;
    i32::from_be_bytes(slice.try_into().unwrap())
}
pub fn read_u32(buffer: &[u8], index: &mut usize) -> u32 {
    let slice = &buffer[*index..*index + 4];
    *index += 4;
    u32::from_be_bytes(slice.try_into().unwrap())
}

pub fn read_u64(buffer: &[u8], index: &mut usize) -> u64 {
    let slice = &buffer[*index..*index + 8];
    *index += 8;
    u64::from_be_bytes(slice.try_into().unwrap())
}
pub fn read_i64(buffer: &[u8], index: &mut usize) -> i64 {
    let slice = &buffer[*index..*index + 8];
    *index += 8;
    i64::from_be_bytes(slice.try_into().unwrap())
}

pub fn read_f64(buffer: &[u8], index: &mut usize) -> f64 {
    let slice = &buffer[*index..*index + 8];
    *index += 8;
    f64::from_be_bytes(slice.try_into().unwrap())
}

pub fn read_string_utf8(
    buffer: &[u8],
    index: &mut usize,
    size: usize,
) -> String {
    // end is min(index+size, buffer len)
    let end = if *index + size > buffer.len() {
        buffer.len()
    } else {
        *index + size
    };
    let slice = &buffer[*index..end];
    *index = end;
    String::from_utf8(slice.to_vec())
        .unwrap_or("⎣INVALID UTF8 STRING⎤".to_string())
}

pub fn read_vec_slice(
    buffer: &[u8],
    index: &mut usize,
    size: usize,
) -> Vec<u8> {
    let slice = &buffer[*index..*index + size];
    *index += size;
    slice.to_vec()
}

pub fn read_slice<'a, const SIZE: usize>(
    buffer: &'a [u8],
    index: &mut usize,
) -> &'a [u8; SIZE] {
    let slice = &buffer[*index..*index + SIZE];
    *index += SIZE;
    slice.try_into().unwrap()
}

/*
write functions: set value at specific index in byte vector, vector length must be big enough
append functions: appends the value at the end of the byte vector, automatically increases size
 */

pub fn write_u8(buffer: &mut [u8], index: &mut usize, val: u8) {
    buffer[*index] = val;
    *index += 1;
}
pub fn append_u8(buffer: &mut Vec<u8>, val: u8) {
    buffer.extend_from_slice(&[val]);
}
pub fn write_i8(buffer: &mut [u8], index: &mut usize, val: i8) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i8(buffer: &mut Vec<u8>, val: i8) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u16(buffer: &mut [u8], index: &mut usize, val: u16) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn write_u32(buffer: &mut [u8], index: &mut usize, val: u32) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}

pub fn set_bit(buffer: &mut [u8], byte_index: usize, bit_position: u8) {
    buffer[byte_index] |= 1 << bit_position;
}

pub fn clear_bit(buffer: &mut [u8], byte_index: usize, bit_position: u8) {
    if byte_index < buffer.len() && bit_position < 8 {
        buffer[byte_index] &= !(1 << bit_position);
    }
}

pub fn toggle_bit(buffer: &mut [u8], byte_index: usize, bit_position: u8) {
    if byte_index < buffer.len() && bit_position < 8 {
        buffer[byte_index] ^= 1 << bit_position;
    }
}

// TODO #161
// pub fn write_int<T: PrimInt>(buffer: &mut Vec<u8>, mut index: usize, val: T) {
//     let bytes = val.to_u128().unwrap().to_le_bytes();
//     for b in bytes {
//         buffer[index] = b;
//         index += 1;
//     }
// }

pub fn append_u16(buffer: &mut Vec<u8>, val: u16) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i16(buffer: &mut [u8], index: &mut usize, val: i16) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i16(buffer: &mut Vec<u8>, val: i16) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn append_u32(buffer: &mut Vec<u8>, val: u32) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i32(buffer: &mut [u8], index: &mut usize, val: i32) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i32(buffer: &mut Vec<u8>, val: i32) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u64(buffer: &mut [u8], index: &mut usize, val: u64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_u64(buffer: &mut Vec<u8>, val: u64) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i64(buffer: &mut [u8], index: &mut usize, val: i64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i64(buffer: &mut Vec<u8>, val: i64) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn append_i128(buffer: &mut Vec<u8>, val: i128) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn append_u128(buffer: &mut Vec<u8>, val: u128) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_f64(buffer: &mut [u8], index: &mut usize, val: f64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_f64(buffer: &mut Vec<u8>, val: f64) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn append_f32(buffer: &mut Vec<u8>, val: f32) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn append_string_utf8(buffer: &mut Vec<u8>, val: &str) {
    buffer.extend_from_slice(val.as_bytes());
}

// hex - buffer conversions

pub fn buffer_to_hex(buffer: Vec<u8>) -> String {
    let n = buffer.len();

    let mut s = String::with_capacity(2 * n);
    for byte in buffer {
        core::write!(s, "{byte:02X}").expect("could not parse buffer")
    }
    s
}

/**
 * seperator: char sequence inserted between each byte
 * pad_size_bytes: if 0, it is ignored
 * x_shorthand: collapse multiple 0 bytes to "xC", with C being the number of zero bytes
 */
pub fn buffer_to_hex_advanced(
    buffer: Vec<u8>,
    seperator: &str,
    pad_size_bytes: usize,
    x_shorthand: bool,
) -> String {
    let n = if pad_size_bytes == 0 {
        buffer.len()
    } else {
        pad_size_bytes
    };

    let buf_len = buffer.len();

    let mut s = String::with_capacity(2 * n);
    let mut i = 0;
    while i < n {
        // next byte
        let byte = if i < buf_len { buffer[i] } else { 0 };
        i += 1;
        // multiple (>=2) zero bytes - x shorthand
        if x_shorthand
            && byte == 0
            && i < n
            && if i < buf_len { buffer[i] } else { 0 } == 0
        {
            let mut zero_count: u8 = 2;
            let initial_i = i;
            while i + 1 < n && buffer[i + 1] == 0 {
                i += 1;
                zero_count += 1;
            }
            // 0 count, max 15
            if zero_count <= 0xf {
                i += 1;
                core::write!(s, "x{zero_count:01X}").expect("could not parse buffer");
            } else {
                i = initial_i;
                core::write!(s, "{byte:02X}").expect("could not parse buffer");
            }
        }
        // normal
        else {
            core::write!(s, "{byte:02X}").expect("could not parse buffer");
        }

        // seperator?
        if !seperator.is_empty() && i < n {
            s += seperator;
        }
    }

    s
}

pub fn hex_to_buffer(hex: String) -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();

    for chunk in &hex.chars().chunks(2) {
        buffer.push(
            u8::from_str_radix(&String::from_iter(chunk), 16)
                .expect("invalid hex buffer"),
        );
    }

    buffer
}

pub fn hex_to_buffer_advanced(hex: String, seperator: &str) -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();

    let raw_hex = hex.replace(seperator, "");

    for chunk in &raw_hex.chars().chunks(2) {
        let part = &String::from_iter(chunk);
        if part.starts_with("x") {
            let count = u8::from_str_radix(part.split_at(1).1, 16)
                .expect("invalid x shortcut");
            for _i in 0..count {
                buffer.push(0);
            }
        } else {
            buffer.push(
                u8::from_str_radix(part, 16).expect("invalid hex buffer"),
            );
        }
    }

    buffer
}

#[cfg(test)]
mod tests {
    use super::{
        buffer_to_hex, buffer_to_hex_advanced, hex_to_buffer,
        hex_to_buffer_advanced,
    };

    /**
     * test byte array to hex string conversion, including seperator characters and fixed length padding
     */
    #[test]
    pub fn buffer_to_hex_tests() {
        assert_eq!(buffer_to_hex_advanced(vec![], "_", 0, true), "");
        assert_eq!(
            buffer_to_hex_advanced(vec![0x00, 0x00, 0x00], "", 0, true),
            "x3"
        );
        assert_eq!(
            buffer_to_hex_advanced(
                vec![
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00
                ],
                "",
                0,
                true
            ),
            "xF"
        );
        assert_eq!(
            buffer_to_hex_advanced(
                vec![
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xaa
                ],
                "",
                0,
                true
            ),
            "00xFAA"
        );
        assert_eq!(buffer_to_hex(vec![0xaa, 0xbb, 0xcc, 0x00]), "AABBCC00");
        assert_eq!(
            buffer_to_hex_advanced(vec![0xaa, 0xbb, 0xcc, 0x00], "-", 0, false),
            "AA-BB-CC-00"
        );
        assert_eq!(
            buffer_to_hex_advanced(
                vec![0xaa, 0xbb, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x01],
                "_",
                0,
                false
            ),
            "AA_BB_CC_00_00_00_00_01"
        );
        assert_eq!(
            buffer_to_hex_advanced(
                vec![0xaa, 0xbb, 0xcc, 0x00, 0x00, 0x00, 0x00, 0x01],
                "_",
                0,
                true
            ),
            "AA_BB_CC_x4_01"
        );

        assert_eq!(
            buffer_to_hex_advanced(vec![0xaa, 0xbb], "-", 4, true),
            "AA-BB-x2"
        );
        assert_eq!(
            buffer_to_hex_advanced(vec![0xaa, 0xbb, 0xcc], "-", 6, false),
            "AA-BB-CC-00-00-00"
        );
        assert_eq!(
            buffer_to_hex_advanced(vec![0xaa, 0xbb, 0xcc, 0xdd], "-", 2, false),
            "AA-BB"
        );
    }

    /**
     * test hex string to byte array conversion, and conversion back to hex string
     */
    #[test]
    pub fn hex_to_buffer_tests() {
        assert_eq!(hex_to_buffer(buffer_to_hex(vec![0x1])), vec![0x1]);
        assert_eq!(
            hex_to_buffer(buffer_to_hex(vec![0xaa, 0xbb, 0xcc, 0x00])),
            vec![0xaa, 0xbb, 0xcc, 0x00]
        );

        assert_eq!(buffer_to_hex(hex_to_buffer("".to_string())), "");
        assert_eq!(
            buffer_to_hex(hex_to_buffer("AABB1122".to_string())),
            "AABB1122"
        );
        assert_eq!(
            buffer_to_hex(hex_to_buffer_advanced(
                "AA-BB-11-22".to_string(),
                "-"
            )),
            "AABB1122"
        );
        assert_eq!(
            buffer_to_hex_advanced(
                hex_to_buffer_advanced("AA-BB-11-22".to_string(), "-"),
                "-",
                0,
                false
            ),
            "AA-BB-11-22"
        );

        assert_eq!(
            hex_to_buffer_advanced("AA-BB-11-22".to_string(), "-"),
            vec![0xAA, 0xBB, 0x11, 0x22]
        );
        assert_eq!(
            hex_to_buffer_advanced("AABB1122".to_string(), ""),
            vec![0xAA, 0xBB, 0x11, 0x22]
        );
    }
}
