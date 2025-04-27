use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, LitStr};

#[proc_macro_attribute]
pub fn auto_open_wrapper(attr: TokenStream, item: TokenStream) -> TokenStream {
    let internal_fn_name = parse_macro_input!(attr as LitStr).value();
    let input_fn = parse_macro_input!(item as ItemFn);

    let vis = &input_fn.vis;
    let sig = &input_fn.sig;

    let expanded = quote! {
        #vis #sig {
            self.set_state(ComInterfaceState::Connecting);
            let res = self.#internal_fn_name().await;
            if res.is_ok() {
                self.set_state(ComInterfaceState::Connected);
            } else {
                self.set_state(ComInterfaceState::NotConnected);
            }
            res
        }
    };

    expanded.into()
}
