use proc_macro::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ImplItemFn, ItemFn, ItemImpl, LitStr};

// #[proc_macro_attribute]
// pub fn auto_open_wrapper(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let internal_fn_name = parse_macro_input!(attr as LitStr).value();
//     let input_fn = parse_macro_input!(item as ItemFn);

//     let vis = &input_fn.vis;
//     let sig = &input_fn.sig;

//     let expanded = quote! {
//         #vis #sig {
//             self.set_state(ComInterfaceState::Connecting);
//             let res = self.#internal_fn_name().await;
//             if res.is_ok() {
//                 self.set_state(ComInterfaceState::Connected);
//             } else {
//                 self.set_state(ComInterfaceState::NotConnected);
//             }
//             res
//         }
//     };

//     expanded.into()
// }

#[proc_macro_attribute]
pub fn create_opener(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ImplItemFn);
    let return_type = &input_fn.sig.output;
    let open_handler = &input_fn.sig.ident;
    let body = input_fn.block;
    // if public throw compile errror
    if input_fn
        .vis
        .to_token_stream()
        .to_string()
        .starts_with("pub")
    {
        panic!("The function is public. Remove the public modifier",);
    }

    let expanded = quote! {
        pub async fn internal_open(&mut self) #return_type {
            #body
        }
        pub async fn #open_handler(&mut self) #return_type {
            self.set_state(ComInterfaceState::Connecting);
            let res = self.internal_open().await;
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
