use std::{env, path::PathBuf, str::FromStr};

use crate::datex_proxy::data::StructureAttributes;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Path, PathSegment, punctuated::Punctuated};

/// Gets the absolute file path of the source file where the macro is invoked.
pub fn get_project_relative_file_path() -> PathBuf {
    let root_path = PathBuf::from_str(
        &env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()),
    )
    .unwrap();
    let call_site = PathBuf::from(Span::call_site().file());

    if call_site.is_relative() {
        return call_site;
    }
    call_site.strip_prefix(&root_path).unwrap_or_else(|_x| {
        panic!(
            "Failed to get project relative file path. Call site: {:?}, Root path: {:?}",
            call_site, root_path
        )
    }).to_path_buf()
}

/// Tries to resolve the datex-core crate to a resolvable name in the current context.
pub fn get_datex_core_crate_name() -> Path {
    let found = match crate_name("datex-core") {
        Ok(found) => found,
        Err(_) =>
        // TODO: decide which namespace to use, for now, fall back to datex-embedded
        {
            return Path {
                leading_colon: None,
                segments: Punctuated::from_iter([
                    PathSegment::from(format_ident!("datex_embedded")),
                    PathSegment::from(format_ident!("core")),
                ]),
            };
        }
    };
    match found {
        FoundCrate::Itself => PathSegment::from(format_ident!("crate")).into(),
        FoundCrate::Name(name) => {
            PathSegment::from(Ident::new(&name, Span::call_site())).into()
        }
    }
}

pub fn derive_datex_prelude(force_datex_core_namespace: bool) -> TokenStream {
    let datex_core_crate_name = get_datex_core_crate_name_with_options(force_datex_core_namespace);
    let is_crate_internal =
        datex_core_crate_name.to_token_stream().to_string() == "crate";

    // only add ast imports if the "ast" feature is enabled
    let derive_prelude_ast = cfg_select! {
        feature = "ast" => {
            if is_crate_internal {
                quote! {
                    use #datex_core_crate_name::preludes::derive_prelude_ast;
                    derive_prelude_ast!();
                }
            } else {
                quote! {
                    use #datex_core_crate_name::derive_prelude_ast;
                    derive_prelude_ast!();
                }
            }
        }
        _ => quote! {},
    };

    let derive_prelude = if is_crate_internal {
        quote! {
            use #datex_core_crate_name::preludes::derive_prelude;
            derive_prelude!();
        }
    } else {
        quote! {
            use #datex_core_crate_name::derive_prelude;
            derive_prelude!();
        }
    };

    quote! {
        #derive_prelude_ast
        #derive_prelude
    }
}

pub fn get_datex_core_crate_name_with_options(
    force_datex_core_namespace: bool,
) -> Path {
    if force_datex_core_namespace {
        PathSegment::from(Ident::new("datex_core", Span::call_site())).into()
    } else {
        get_datex_core_crate_name()
    }
}
