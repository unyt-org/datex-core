use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Ident, ImplItemFn, ItemFn, ItemImpl,
    LitStr,
};

pub fn create_opener_impl(original_open: ImplItemFn) -> TokenStream {
    let return_type = &original_open.sig.output;
    let original_name = &original_open.sig.ident;
    let original_body = original_open.block;

    let new_internal_name =
        Ident::new(&format!("internal_{}", original_name), Span::call_site());

    if original_open
        .vis
        .to_token_stream()
        .to_string()
        .starts_with("pub")
    {
        panic!("The function is public. Remove the public modifier",);
    }

    let expanded = quote! {
        pub async fn #new_internal_name(&mut self) #return_type {
            #original_body
        }
        pub async fn #original_name(&mut self) #return_type {
            self.set_state(ComInterfaceState::Connecting);
            let res = self.#new_internal_name().await;
            if res.is_ok() {
                self.set_state(ComInterfaceState::Connected);
            } else {
                self.set_state(ComInterfaceState::NotConnected);
            }
            res
        }
    };

    return TokenStream::from(expanded);
}
