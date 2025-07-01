use crate::stdlib::usize;

use crate::runtime::global_context::get_global_context;

pub fn random_bytes_slice<const SIZE: usize>() -> [u8; SIZE] {
    let crypto = get_global_context().crypto;
    let crypto = crypto.lock().unwrap();
    crypto.random_bytes(SIZE).try_into().unwrap()
}
pub fn random_bytes(size: usize) -> Vec<u8> {
    let crypto = get_global_context().crypto;
    let crypto = crypto.lock().unwrap();
    crypto.random_bytes(size)
}
