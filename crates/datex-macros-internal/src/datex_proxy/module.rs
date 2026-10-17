use crate::{
    datex_proxy::native_callable::generate_native_callable_from_fn,
    utils::{derive_datex_prelude, get_datex_core_crate_name},
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Item, ItemMod, LitStr, parse::Parse};

struct ModuleAttributes {
    name: Option<String>,
}

impl Parse for ModuleAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;

            if ident == "name" {
                name = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    "unknown datex attribute",
                ));
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        Ok(Self {
            name: name.map(|lit| lit.value()),
        })
    }
}

pub fn generate_mod_glue_code(
    args: Option<TokenStream>,
    item: &ItemMod,
) -> TokenStream {
    let items = if let Some((_, items)) = &item.content {
        items
    } else {
        panic!("The #[datex] attribute can only be applied to inline modules.");
    };

    let datex_core_crate_name = get_datex_core_crate_name();
    let attrs = &item.attrs;
    let module_attributes: Option<ModuleAttributes> =
        args.map(|args| syn::parse2(args).unwrap());

    // datex module name either #[datex(name = "custom_name")] or the module name itself
    let datex_module_name = module_attributes
        .and_then(|attrs| attrs.name)
        .unwrap_or_else(|| item.ident.to_string());

    let vis = &item.vis;
    let ident = &item.ident;

    // map items to their token streams
    let datex_items: Vec<TokenStream> =
        items.iter().filter_map(map_to_datex_item).collect();
    let prelude = derive_datex_prelude(false);
    quote! {
        #(#attrs)*
        #vis mod #ident {
            #prelude

            #(#items)*

            #datex_core_crate_name::inventory::submit! {
                #datex_core_crate_name::datex_registry::DatexModuleRegistration {
                    name: #datex_module_name,
                    create_module: |cache| #datex_core_crate_name::values::core_values::map::Map::new_structural(
                        vec![#(#datex_items),*],
                    ),
                }
            }
        }
    }
}

fn map_to_datex_item(item: &Item) -> Option<TokenStream> {
    match item {
        // only register pub functions
        Item::Fn(item_fn)
            if item_fn.vis.to_token_stream().to_string().contains("pub") =>
        {
            let mapped = generate_native_callable_from_fn(item_fn);
            let name = item_fn.sig.ident.to_string();

            Some(quote! {
                (
                    #name.into(),
                    #mapped.into(),
                )
            })
        }
        _ => None,
    }
}
