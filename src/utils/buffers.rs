use byteorder::{LittleEndian, ReadBytesExt};
use core::fmt::Write;
use itertools::Itertools;

/*
read functions for primitive data types on a u8 array, also increments the index
 */

pub fn read_u8(buffer: &[u8], index: &mut usize) -> u8 {
    let val = buffer[*index];
    *index += 1;
    return val;
}

pub fn read_i8(buffer: &[u8], index: &mut usize) -> i8 {
    let mut slice = &buffer[*index..*index + 1];
    *index += 1;
    return slice.read_i8().unwrap();
}

pub fn read_u16(buffer: &[u8], index: &mut usize) -> u16 {
    let mut slice = &buffer[*index..*index + 2];
    *index += 2;
    return slice.read_u16::<LittleEndian>().unwrap();
}
pub fn read_i16(buffer: &[u8], index: &mut usize) -> i16 {
    let mut slice = &buffer[*index..*index + 2];
    *index += 2;
    return slice.read_i16::<LittleEndian>().unwrap();
}

pub fn read_i32(buffer: &[u8], index: &mut usize) -> i32 {
    let mut slice = &buffer[*index..*index + 4];
    *index += 4;
    return slice.read_i32::<LittleEndian>().unwrap();
}
pub fn read_u32(buffer: &[u8], index: &mut usize) -> u32 {
    let mut slice = &buffer[*index..*index + 4];
    *index += 4;
    return slice.read_u32::<LittleEndian>().unwrap();
}

pub fn read_u64(buffer: &[u8], index: &mut usize) -> u64 {
    let mut slice = &buffer[*index..*index + 8];
    *index += 8;
    return slice.read_u64::<LittleEndian>().unwrap();
}
pub fn read_i64(buffer: &[u8], index: &mut usize) -> i64 {
    let mut slice = &buffer[*index..*index + 8];
    *index += 8;
    return slice.read_i64::<LittleEndian>().unwrap();
}

pub fn read_f64(buffer: &[u8], index: &mut usize) -> f64 {
    let mut slice = &buffer[*index..*index + 8];
    *index += 8;
    return slice.read_f64::<LittleEndian>().unwrap();
}

pub fn read_string_utf8(buffer: &[u8], index: &mut usize, size: usize) -> String {
    // end is min(index+size, buffer len)
    let end = if *index + size > buffer.len() {
        buffer.len()
    } else {
        *index + size
    };
    let slice = &buffer[*index..end];
    *index = end;
    return String::from_utf8(slice.to_vec()).unwrap_or("⎣INVALID UTF8 STRING⎤".to_string());
}

pub fn read_slice(buffer: &[u8], index: &mut usize, size: usize) -> Vec<u8> {
    let slice = &buffer[*index..*index + size];
    *index += size;
    return slice.to_vec();
}

/*
write functions: set value at specific index in byte vector, vector length must be big enough
append functions: appends the value at the end of the byte vector, automatically increases size
 */

pub fn write_u8(buffer: &mut Vec<u8>, index: &mut usize, val: u8) {
    buffer[*index] = val;
    *index += 1;
}
pub fn append_u8(buffer: &mut Vec<u8>, val: u8) {
    buffer.extend_from_slice(&[val]);
}
pub fn write_i8(buffer: &mut Vec<u8>, index: &mut usize, val: i8) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i8(buffer: &mut Vec<u8>, val: i8) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u16(buffer: &mut Vec<u8>, index: &mut usize, val: u16) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_u16(buffer: &mut Vec<u8>, val: u16) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i16(buffer: &mut Vec<u8>, index: &mut usize, val: i16) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i16(buffer: &mut Vec<u8>, val: i16) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u32(buffer: &mut Vec<u8>, index: &mut usize, val: u32) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_u32(buffer: &mut Vec<u8>, val: u32) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i32(buffer: &mut Vec<u8>, index: &mut usize, val: i32) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i32(buffer: &mut Vec<u8>, val: i32) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u64(buffer: &mut Vec<u8>, index: &mut usize, val: u64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_u64(buffer: &mut Vec<u8>, val: u64) {
    buffer.extend_from_slice(&val.to_le_bytes());
}
pub fn write_i64(buffer: &mut Vec<u8>, index: &mut usize, val: i64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_i64(buffer: &mut Vec<u8>, val: i64) {
    buffer.extend_from_slice(&val.to_le_bytes());
}

pub fn write_f64(buffer: &mut Vec<u8>, index: &mut usize, val: f64) {
    let bytes = val.to_le_bytes();
    for b in bytes {
        buffer[*index] = b;
        *index += 1;
    }
}
pub fn append_f64(buffer: &mut Vec<u8>, val: f64) {
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
        write!(s, "{:02X}", byte).expect("could not parse buffer")
    }
    return s;
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
        if x_shorthand && byte == 0 && i < n && if i < buf_len { buffer[i] } else { 0 } == 0 {
            let mut zero_count: u8 = 2;
            let initial_i = i;
            while i + 1 < n && buffer[i + 1] == 0 {
                i += 1;
                zero_count += 1;
            }
            // 0 count, max 15
            if zero_count <= 0xf {
                i += 1;
                write!(s, "x{:01X}", zero_count).expect("could not parse buffer");
            } else {
                i = initial_i;
                write!(s, "{:02X}", byte).expect("could not parse buffer");
            }
        }
        // normal
        else {
            write!(s, "{:02X}", byte).expect("could not parse buffer");
        }

        // seperator?
        if seperator.len() != 0 && i < n {
            s += seperator;
        }
    }

    return s;
}

pub fn hex_to_buffer(hex: String) -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();

    for chunk in &hex.chars().chunks(2) {
        buffer.push(u8::from_str_radix(&String::from_iter(chunk), 16).expect("invalid hex buffer"));
    }

    return buffer;
}

pub fn hex_to_buffer_advanced(hex: String, seperator: &str) -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();

    let raw_hex = hex.replace(seperator, "");

    for chunk in &raw_hex.chars().chunks(2) {
        let part = &String::from_iter(chunk);
        if part.starts_with("x") {
            let count = u8::from_str_radix(part.split_at(1).1, 16).expect("invalid x shortcut");
            for _i in 0..count {
                buffer.push(0);
            }
        } else {
            buffer.push(u8::from_str_radix(part, 16).expect("invalid hex buffer"));
        }
    }

    return buffer;
}
