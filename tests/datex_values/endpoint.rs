use datex_core::{
    datex_values::{Endpoint, EndpointInstance},
};
use datex_core::datex_values::EndpointType;
use crate::context::init_global_context;

#[test]
fn new_random() {
    init_global_context();
    let endpoint = Endpoint::new_random();
    assert_eq!(endpoint.type_, EndpointType::Anonymous);
    assert_eq!(endpoint.instance, EndpointInstance::Any);
}
