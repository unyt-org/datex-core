use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ItemFn;

pub fn generate_fn_glue_code(
    _args: Option<TokenStream>,
    input: TokenStream,
    item: ItemFn,
) -> TokenStream {
    quote! {
        #item
    }
}