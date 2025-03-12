use std::usize;

use crate::runtime::global_context::get_global_context;

// pub fn random_bytes<const SIZE: usize>(size: usize) -> Box<[u8; SIZE]> {
//     let crypto = get_global_context().crypto;
//     let crypto = crypto.lock().unwrap();
//     let slice: <[]>crypto.random_bytes(size)
// }

pub fn random_bytes<const SIZE: usize>() -> [u8; SIZE] {
    let crypto = get_global_context().crypto;
    let crypto = crypto.lock().unwrap();
    crypto.random_bytes(SIZE).try_into().unwrap()
}
