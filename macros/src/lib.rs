use proc_macro::{Span, TokenStream};
use quote::quote;
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

    // Extract the return type and the method name
    let return_type = &input_fn.sig.output;
    let method_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn
        pub async fn open(&mut self) #return_type {
            println!("a");
            let result = self.#method_name().await;
            println!("b");
            result
        }
    };

    return TokenStream::from(expanded);

    // Generate the new method `openPub`
    let expanded = quote! {
        // The original method stays the same
        #input_fn

        // Generate the new openPub method
        pub async fn openPub(&mut self) -> #return_type {
            println!("a");
            let result = self.#method_name().await;
            println!("b");
            result
        }
    };

    // Return the generated code as TokenStream
    TokenStream::from(expanded)
}

// #[proc_macro]
// pub fn create_open_pub(input: TokenStream) -> TokenStream {
//     // Parse the input tokens as an `ItemImpl` (which represents an implementation block)
//     let input = parse_macro_input!(input as ItemImpl);
//     return TokenStream::new();

//     // Find the `open` method in the `impl` block
//     if let Some(open_method) = input.items.iter().find_map(|item| {
//         if let syn::ImplItem::Fn(m) = item {
//             if m.sig.ident == "open" {
//                 Some(m)
//             } else {
//                 None
//             }
//         } else {
//             None
//         }
//     }) {
//         // Extract the return type of the `open` method
//         let return_type = &open_method.sig.output;
//         let open_fn_name = &open_method.sig.ident;

//         // Generate the code for the new method `openPub`
//         // let expanded = quote! {
//         //     pub async fn openPub(&mut self) -> #return_type {
//         //         println!("a");
//         //         let result = self.#open_fn_name().await;
//         //         println!("b");
//         //         result
//         //     }
//         // };
//         TokenStream::new()
//         // Return the generated code
//         // TokenStream::from(expanded)
//     } else {
//         // If no `open` method was found, return an empty TokenStream (no code generated)
//         TokenStream::new()
//     }
// }
