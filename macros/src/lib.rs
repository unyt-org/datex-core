use com_interface_macros::create_opener_impl;
use proc_macro::TokenStream;
use syn::{ImplItemFn, ItemImpl, parse_macro_input};
mod bitfield_macros;
mod com_interface_macros;
mod value_macros;
/// This macro is used to create an opener for a interface.
/// ```ignore
/// # use datex_macros::create_opener;
/// # struct MyInterface;
/// # struct MyError;
/// impl MyInterface {
///     #[create_opener]
///     async fn open(&mut self) -> Result<(), MyError> {
///         // ...
///         # Ok(())
///     }
/// }
/// ```
/// The macro will move the original function (let's call it `open`) to a new function called `internal_open`
/// and create a new function called `open` that will call `internal_open` but handle also the state of the interface automatically.
/// The original function will remain private and the new function will be public.
#[proc_macro_attribute]
pub fn create_opener(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let original_open = parse_macro_input!(item as ImplItemFn);
    create_opener_impl(original_open).into()
}

/// This macro is used to create a COM interface.
/// It will add the following methods to the interface:
/// - `destroy`: This method is used to destroy the interface.
/// - `destroy_ref`: This method is used to destroy the interface per reference.
#[proc_macro_attribute]
pub fn com_interface(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(input as ItemImpl);
    com_interface_macros::com_interface_impl(input_impl).into()
}

#[proc_macro_derive(FromCoreValue)]
pub fn from_core_value_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    value_macros::from_core_value_derive_impl(input).into()
}

#[proc_macro_derive(DatexStruct)]
pub fn derive_datex_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    value_macros::derive_datex_struct(input).into()
}

#[proc_macro_derive(BitfieldSerde)]
pub fn derive_bitfield_serde(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    bitfield_macros::derive_bitfield_serde(input).into()
}
