use datex_core::{
    datex_values::Endpoint,
    utils::buffers::{buffer_to_hex, buffer_to_hex_advanced, hex_to_buffer},
};

/**
 * test if endpoints are created correctly
 */
#[test]
pub fn create_endpoints() {
    let id_endpoint = Endpoint::new(
        &hex_to_buffer("00112233445566778899AABBCCDDEEFF1122".to_string()),
        Endpoint::ANY_INSTANCE,
    );
    // binary repr
    assert_eq!(id_endpoint.get_binary().len(), 21);
    assert_eq!(
        buffer_to_hex_advanced(id_endpoint.get_binary().to_vec(), " ", 0, false),
        "00 00 11 22 33 44 55 66 77 88 99 AA BB CC DD EE FF 11 22 00 00"
    );
    // string repr
    assert_eq!(
        id_endpoint.to_string(false),
        "@@00112233445566778899AABBCCDDEEFF1122"
    );
    // instance
    assert_eq!(id_endpoint.get_instance(), Endpoint::ANY_INSTANCE);

    let person_endpoint = Endpoint::new_person("theodore_roosevelt", 2);
    // binary repr
    assert_eq!(person_endpoint.get_binary().len(), 21);
    assert_eq!(
        buffer_to_hex_advanced(person_endpoint.get_binary().to_vec(), " ", 0, false),
        "01 74 68 65 6F 64 6F 72 65 5F 72 6F 6F 73 65 76 65 6C 74 02 00"
    );
    // string repr
    assert_eq!(person_endpoint.to_string(false), "@theodore_roosevelt/0002");
    // instance
    assert_eq!(person_endpoint.get_instance(), 2);
}

#[test]
pub fn endpoints_from_binary() {
    let id_endpoint = Endpoint::new(
        &hex_to_buffer("00112233445566778899AABBCCDDEEFF1122".to_string()),
        Endpoint::ANY_INSTANCE,
    );
    // create new
    let cloned_id_endpoint = Endpoint::new_from_binary(id_endpoint.get_binary());
    assert_eq!(
        id_endpoint.to_string(false),
        cloned_id_endpoint.to_string(false)
    );

    let person_endpoint = Endpoint::new_person("leon", 0xaa);
    // create new
    let cloned_person_endpoint = Endpoint::new_from_binary(person_endpoint.get_binary());
    assert_eq!(
        person_endpoint.to_string(false),
        cloned_person_endpoint.to_string(false)
    );
}
