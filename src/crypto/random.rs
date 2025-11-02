use crate::runtime::global_context::get_global_context;
use crate::stdlib::vec::Vec;

pub fn random_bytes_slice<const SIZE: usize>() -> [u8; SIZE] {
    let crypto = get_global_context().crypto;
    crypto.random_bytes(SIZE).try_into().unwrap()
}
pub fn random_bytes(size: usize) -> Vec<u8> {
    let crypto = get_global_context().crypto;
    crypto.random_bytes(size)
}
