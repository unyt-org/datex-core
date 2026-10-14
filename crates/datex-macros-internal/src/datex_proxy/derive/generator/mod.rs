use crate::{
    datex_proxy::{
        data::StructureData,
        generator::{
            classification::generate_classification,
            convert_parts::generate_convert_parts,
            convert_value::generate_convert_value,
            datex_expression_data::generate_datex_expression_data,
            datex_hash::generate_datex_hash,
            datex_native::generate_datex_native,
            datex_type::{generate_core_lib_type_id, generate_datex_type},
            serde_dif::generate_serde_dif,
            to_instructions::generate_to_instructions,
            value_access::generate_value_access,
        },
    },
    generator::iter_parts::generate_child_iterator,
    utils::derive_datex_prelude,
};
use proc_macro2::TokenStream;
use quote::quote;

mod classification;
mod convert_parts;
mod convert_value;
mod datex_expression_data;
mod datex_hash;
mod datex_native;
mod datex_type;
mod helpers;
mod iter_parts;
mod serde_dif;
mod to_instructions;
mod value_access;

/// Generates the code for the derive macro based on the provided structure data.
pub fn generate_derive_code(structure_data: StructureData) -> TokenStream {
    // generate trait impls
    let datex_native = generate_datex_native(&structure_data);
    let convert_parts = generate_convert_parts(&structure_data);
    let datex_type = generate_datex_type(&structure_data);
    let core_lib_type_id = generate_core_lib_type_id(&structure_data);
    let convert_value = generate_convert_value(&structure_data);
    let classification = generate_classification(&structure_data);
    let value_access = generate_value_access(&structure_data);
    let datex_hash = generate_datex_hash(&structure_data);
    let to_instructions = generate_to_instructions(&structure_data);
    let serde_dif = generate_serde_dif(&structure_data);
    let child_iterator = generate_child_iterator(&structure_data);

    let datex_expression_data =
        cfg_select! {
            feature = "ast" => generate_datex_expression_data(&structure_data),
            _ => quote! {},
        };

    let prelude = derive_datex_prelude(structure_data.attributes.force_datex_core_namespace);
    quote! {
        const _: () = {
            #prelude

            use core::fmt;

            #datex_native
            #convert_parts
            #datex_type
            #core_lib_type_id
            #convert_value
            #classification
            #value_access
            #datex_hash
            #to_instructions
            #serde_dif
            #datex_expression_data
            #child_iterator
        };
    }
}
