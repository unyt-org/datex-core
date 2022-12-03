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



// // get hex string id from buffer
// fn buffer2hex(buffer:Uint8Array|ArrayBuffer, seperator?:string, pad_size_bytes?:number, x_shorthand = false):string {
//     if (buffer instanceof ArrayBuffer) buffer = new Uint8Array(buffer);

//     // first pad buffer
//     if (pad_size_bytes) buffer = buffer.slice(0, pad_size_bytes);

//     let array:string[] = <string[]> Array.prototype.map.call(buffer, x => ('00' + x.toString(16).toUpperCase()).slice(-2))
//     let skipped_bytes = 0;

//     // collapse multiple 0s to x...
//     if (x_shorthand) {
//         array = array.slice(0,pad_size_bytes).reduce((previous, current) => {
//             if (current == '00') {
//                 if (previous.endsWith('00')) {
//                     skipped_bytes++;
//                     return previous.slice(0, -2) + "x2"; // add to existing 00
//                 }
//                 else if (previous[previous.length-2] == 'x') {
//                     const count = (parseInt(previous[previous.length-1],16)+1);
//                     if (count <= 0xf) {
//                         skipped_bytes++;
//                         return previous.slice(0, -1) + count.toString(16).toUpperCase()  // add to existing x... max 15
//                     }
//                 }
//             }
//             return previous + current;
//         }).split(/(..)/g).filter(s=>!!s);
//     }

//     if (pad_size_bytes != undefined) array = Array.from({...array, length: pad_size_bytes-skipped_bytes}, x=>x==undefined?'00':x); // pad

//     return array.join(seperator??'');
// }