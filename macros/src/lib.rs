use com_interface::create_opener_impl;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ImplItemFn};
mod com_interface;

/// This macro is used to create an opener for a interface.
/// ```
/// impl MyInterface {
///     #[create_opener]
///     async fn open(&mut self) -> Result<(), MyError> {
///         // Your implementation here
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
