use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Ident, ImplItemFn, ItemImpl};
pub fn com_interface_impl(input_impl: ItemImpl) -> TokenStream {
    let name = &input_impl.self_ty;
    let method_content = quote! {
        /// This method is used to destroy the interface.
        /// It will close the socket and set the state to Destroyed.
        pub async fn destroy(mut self) {
            self.handle_destroy().await;
        }

        /// This method is used to destroy the interface per reference.
        /// It will close the socket and set the state to Destroyed.
        /// It is used when the interface is used in a context where
        /// the interface is not owned by the caller.
        pub async fn destroy_ref(&mut self) {
            self.handle_destroy().await;
        }
    };

    // Generate the expanded code by appending the new method
    let expanded = quote! {
        #input_impl

        impl #name {
            #method_content
        }
    };

    // Convert the generated code into a TokenStream
    expanded
}
pub fn create_opener_impl(original_open: ImplItemFn) -> TokenStream {
    let return_type = &original_open.sig.output;
    let original_name = &original_open.sig.ident;
    let original_body = original_open.block;

    let new_internal_name =
        Ident::new(&format!("internal_{original_name}"), Span::call_site());

    if original_open
        .vis
        .to_token_stream()
        .to_string()
        .starts_with("pub")
    {
        panic!("The function is public. Remove the public modifier",);
    }
    let expanded = {
        if original_open.sig.asyncness.is_some() {
            quote! {
                async fn #new_internal_name(&mut self) #return_type {
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
            }
        } else {
            quote! {
                fn #new_internal_name(&mut self) #return_type {
                    #original_body
                }
                pub fn #original_name(&mut self) #return_type {
                    self.set_state(ComInterfaceState::Connecting);
                    let res = self.#new_internal_name();
                    if res.is_ok() {
                        self.set_state(ComInterfaceState::Connected);
                    } else {
                        self.set_state(ComInterfaceState::NotConnected);
                    }
                    res
                }
            }
        }
    };

    expanded
}
