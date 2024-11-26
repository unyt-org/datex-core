use datex_core::utils::{buffers::{buffer_to_hex, buffer_to_hex_advanced, hex_to_buffer, hex_to_buffer_advanced}};


/**
 * test byte array to hex string conversion, including seperator characters and fixed length padding
 */
#[test]
pub fn buffer_to_hex_tests() {

    assert_eq!(buffer_to_hex_advanced(vec![], "_", 0, true),  "");
    assert_eq!(buffer_to_hex_advanced(vec![0x00,0x00,0x00], "", 0, true),  "x3");
    assert_eq!(buffer_to_hex_advanced(vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00], "", 0, true),  "xF");
    assert_eq!(buffer_to_hex_advanced(vec![0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xaa], "", 0, true),  "00xFAA");
    assert_eq!(buffer_to_hex(vec![0xaa,0xbb,0xcc,0x00]), "AABBCC00");
    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb,0xcc,0x00], "-", 0, false), "AA-BB-CC-00");
    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb,0xcc,0x00,0x00,0x00,0x00,0x01], "_", 0, false), "AA_BB_CC_00_00_00_00_01");
    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb,0xcc,0x00,0x00,0x00,0x00,0x01], "_", 0, true),  "AA_BB_CC_x4_01");

    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb], "-", 4, true),  "AA-BB-x2");
    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb,0xcc], "-", 6, false),  "AA-BB-CC-00-00-00");
    assert_eq!(buffer_to_hex_advanced(vec![0xaa,0xbb,0xcc,0xdd], "-", 2, false),  "AA-BB");

}

/**
 * test hex string to byte array conversion, and conversion back to hex string
 */
#[test]
pub fn hex_to_buffer_tests() {

    assert_eq!(hex_to_buffer(buffer_to_hex(vec![0x1])),  vec![0x1]);
    assert_eq!(hex_to_buffer(buffer_to_hex(vec![0xaa,0xbb,0xcc,0x00])),  vec![0xaa,0xbb,0xcc,0x00]);

    assert_eq!(buffer_to_hex(hex_to_buffer("".to_string())),  "");
    assert_eq!(buffer_to_hex(hex_to_buffer("AABB1122".to_string())),  "AABB1122");
    assert_eq!(buffer_to_hex(hex_to_buffer_advanced("AA-BB-11-22".to_string(), "-")),  "AABB1122");
    assert_eq!(buffer_to_hex_advanced(hex_to_buffer_advanced("AA-BB-11-22".to_string(), "-"), "-", 0, false),  "AA-BB-11-22");

    assert_eq!(hex_to_buffer_advanced("AA-BB-11-22".to_string(), "-"),vec![0xAA,0xBB,0x11,0x22] );
    assert_eq!(hex_to_buffer_advanced("AABB1122".to_string(), ""),vec![0xAA,0xBB,0x11,0x22] );

}

/**
 * demo test that fails
 */
#[test]
pub fn failing_test() {
    assert_eq!(1,2);
}