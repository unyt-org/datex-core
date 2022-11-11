use byteorder::{LittleEndian, ReadBytesExt};

/*
read functions for primitive data types on a u8 array, also increments the index
 */

pub fn read_u8(buffer: &[u8], index: &mut usize) -> u8 {
	let val = buffer[*index];
	*index += 1;
	return val;
}
pub fn read_i8(buffer: &[u8], index: &mut usize) -> i8 {
	let mut slice = &buffer[*index..*index+1];
	*index += 1;
	return slice.read_i8().unwrap();
}

pub fn read_u16(buffer: &[u8], index: &mut usize) -> u16 {
	let mut slice = &buffer[*index..*index+2];
	*index += 2;
	return slice.read_u16::<LittleEndian>().unwrap();
}
pub fn read_i16(buffer: &[u8], index: &mut usize) -> i16 {
	let mut slice = &buffer[*index..*index+2];
	*index += 2;
	return slice.read_i16::<LittleEndian>().unwrap();
}

pub fn read_i32(buffer: &[u8], index: &mut usize) -> i32 {
	let mut slice = &buffer[*index..*index+4];
	*index += 4;
	return slice.read_i32::<LittleEndian>().unwrap();
}
pub fn read_u32(buffer: &[u8], index: &mut usize) -> u32 {
	let mut slice = &buffer[*index..*index+4];
	*index += 4;
	return slice.read_u32::<LittleEndian>().unwrap();
}

pub fn read_u64(buffer: &[u8], index: &mut usize) -> u64 {
	let mut slice = &buffer[*index..*index+8];
	*index += 8;
	return slice.read_u64::<LittleEndian>().unwrap();
}
pub fn read_i64(buffer: &[u8], index: &mut usize) -> i64 {
	let mut slice = &buffer[*index..*index+8];
	*index += 8;
	return slice.read_i64::<LittleEndian>().unwrap();
}

pub fn read_f64(buffer: &[u8], index: &mut usize) -> f64 {
	let mut slice = &buffer[*index..*index+8];
	*index += 8;
	return slice.read_f64::<LittleEndian>().unwrap();
}


pub fn read_string_utf8(buffer: &[u8], index: &mut usize, size: usize) -> String {
	let slice = &buffer[*index..*index+size];
	*index += size;
	return String::from_utf8(slice.to_vec()).expect("could not read string");
}

pub fn read_slice(buffer: &[u8], index: &mut usize, size: usize) -> Vec<u8> {
	let slice = &buffer[*index..*index+size];
	*index += size;
	return slice.to_vec();
}