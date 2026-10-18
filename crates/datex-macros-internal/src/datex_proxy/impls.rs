use crate::{
    datex_proxy::native_callable::generate_native_callable_from_impl_fn,
    utils::{
        derive_datex_prelude, get_datex_core_crate_name,
        get_project_relative_file_path,
    },
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ItemImpl;

pub fn generate_impl_glue_code(
    _args: Option<TokenStream>,
    input: TokenStream,
    mut item: ItemImpl,
) -> TokenStream {
    let datex_core_crate_name = get_datex_core_crate_name();
    let self_ty = &item.self_ty;

    let mut methods = Vec::new();
    let mut static_methods = Vec::new();

    for impl_item in &mut item.items {
        if let syn::ImplItem::Fn(method) = impl_item {
            let is_method = method
                .sig
                .inputs
                .first()
                .map(|arg| matches!(arg, syn::FnArg::Receiver(_)))
                .unwrap_or(false);
            let callable =
                generate_native_callable_from_impl_fn(method, self_ty);

            if is_method {
                methods.push(quote! {
                    EntityImplMethod {
                        call_on_owner: true,
                        callable: #callable,
                    }
                })
            } else {
                static_methods.push(quote! {
                    #callable
                })
            }
        }
    }

    let namespace = {
        let mut ns = get_project_relative_file_path();
        ns.set_extension("");
        ns.to_str()
            .expect("Failed to convert file path to string")
            .to_string()
    };
    let name = item.self_ty.to_token_stream().to_string();
    let prelude = derive_datex_prelude(false);

    quote! {
        #input

        const _: () = {
            #prelude

            #datex_core_crate_name::inventory::submit! {
                #datex_core_crate_name::datex_registry::DatexImplRegistration {
                    namespace: #namespace,
                    name: #name,
                    create_impl: |cache| EntityImpl {
                        methods: vec![#(#methods),*],
                        static_methods: vec![#(#static_methods),*],
                    },
                    owner_type_id: || { ::core::any::TypeId::of::<#self_ty>()},
                }
            }
        };
    }
}
