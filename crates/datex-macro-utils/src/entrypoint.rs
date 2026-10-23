use std::path::{Path, PathBuf};

use crate::get_absolute_file_path;
use datex_core::{
    compiler::{CompileOptions, compile_template},
    runtime::{Runtime, RuntimeConfig},
    traits::convert_value_container::ConvertValueContainer,
    values::value_container::ValueContainer,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, FnArg, Ident, ItemFn, LitStr, Pat, PatIdent, Token, Type,
    parse::{Parse, ParseStream},
};
use datex_core::compiler::compilation_result::CompilationResult;

#[derive(Debug)]
pub struct ParsedAttributes {
    pub config: Option<PathBuf>,
}

impl Parse for ParsedAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut config = None;

        let source_file = get_absolute_file_path();

        // first try if directly a path string
        if let Ok(config_path) = get_config_path(&input, &source_file) {
            return Ok(ParsedAttributes {
                config: Some(config_path),
            });
        }

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if ident == "config" {
                config = Some(get_config_path(&input, &source_file)?);
            } else {
                return Err(input.error("Unknown attribute"));
            }

            // optionally parse comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ParsedAttributes { config })
    }
}

fn get_config_path(
    input: &ParseStream,
    source_file: &Path,
) -> Result<PathBuf, syn::Error> {
    if input.peek(LitStr) {
        if let syn::Lit::Str(litstr) = input.parse()? {
            let config_path_str = litstr.value();
            let path = source_file
                .parent()
                .unwrap()
                .join(config_path_str)
                .canonicalize();
            if let Ok(path) = path {
                Ok(path)
            } else {
                Err(input.error(path.unwrap_err().to_string()))
            }
        } else {
            Err(input
                .error("Invalid value for `config` - must be a path string"))
        }
    } else {
        Err(input.error("Not a string"))
    }
}

pub struct DatexMainInput<'a> {
    /// Attributes of the main macro, e.g. config path
    pub parsed_attributes: ParsedAttributes,
    /// The function annotated with the macro, containing the application logic
    pub func: ItemFn,
    /// Custom namespace for datex_core
    pub datex_core_namespace: &'a str,
    /// Optional setup code to run before creating the runtime, e.g. for setting environment variables
    pub setup: Option<TokenStream>,
    /// Optional initialization code to run after creating, but before starting the runtime.
    /// Has access to the [Runtime] via "runtime" variable. Any variables created here will be dropped before entering the main function body.
    pub init_scoped: Option<TokenStream>,
    /// Optional initialization code to run after creating, but before starting the runtime.
    /// Has access to the [Runtime] via "runtime" variable. Any variables created here can be accessed in the main function body.
    pub init_unscoped: Option<TokenStream>,
    /// Optional code to run before the main function body, after the runtime has been started
    pub pre_body: Option<TokenStream>,
    /// Additional attributes to add to the generated main function
    pub additional_attributes: Vec<Attribute>,
    /// Custom input arguments for the main function, e.g. for providing additional dependencies
    pub custom_main_inputs: Vec<FnArg>,
    /// Whether to enforce that the main function is named `main`
    pub enforce_main_name: bool,
}

/// Main implementation function for the datex_main macro
pub fn datex_main_impl(input: DatexMainInput) -> TokenStream {
    let config = get_config(&input.parsed_attributes);
    datex_main_impl_with_config(input, config)
}

/// Main implementation function for the datex_main macro, with a provided config
pub fn datex_main_impl_with_config(
    input: DatexMainInput,
    config: Option<RuntimeConfig>,
) -> TokenStream {
    let config_bytes = get_config_compiled_token_stream(config);

    if input.func.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            input.func.sig.fn_token,
            "the function must be async",
        )
        .to_compile_error();
    }

    if input.enforce_main_name && input.func.sig.ident != "main" {
        return syn::Error::new_spanned(
            &input.func.sig.ident,
            "the function must be named `main`",
        )
        .to_compile_error();
    }

    let (runtime_arg_ident, runtime_arg_type) = match get_arg_ident_and_type(
        0,
        &input.func,
        "expected an identifier argument like `runtime: Runtime`",
    ) {
        Ok(ident) => ident,
        Err(err) => return err.to_compile_error(),
    };

    let ItemFn {
        mut sig,
        vis,
        block: body,
        attrs,
    } = input.func;

    sig.inputs.clear();
    for input in input.custom_main_inputs {
        sig.inputs.push(input);
    }

    let core_namespace =
        syn::parse_str::<syn::Path>(input.datex_core_namespace)
            .expect("invalid datex_core namespace");

    let additional_attributes = input.additional_attributes;
    let setup = input.setup;
    let init_scoped = input.init_scoped;
    let init_unscoped = input.init_unscoped;
    let pre_body = input.pre_body;

    let runtime_clone = if init_unscoped.is_some() || init_scoped.is_some() {
        quote! { let runtime = runner.runtime.clone(); }
    } else {
        quote! {}
    };

    quote! {
        #(#additional_attributes)*
        #(#attrs)*
        #vis #sig {
            use #core_namespace::{
                runtime::{RuntimeRunner, RuntimeConfig, Runtime},
                traits::convert_value_container::ConvertValueContainer,
            };

            #setup

            let config = match (#config_bytes) {
                Some(bytes) => ConvertValueContainer::try_from_bytes(bytes, &Runtime::stub()).unwrap(),
                None => RuntimeConfig::default(),
            };

            let runner = RuntimeRunner::new(config);
            #runtime_clone
            {
                #init_scoped
            }
            #init_unscoped
            runner.run(async move |#runtime_arg_ident: #runtime_arg_type| {
                #pre_body
                {
                    #body
                }
            }).await
        }
    }
}

pub fn get_config(parsed_attr: &ParsedAttributes) -> Option<RuntimeConfig> {
    // try to get config from config path
    parsed_attr.config.as_ref().map(|path| {
        RuntimeConfig::try_from_dx_file(path, &Runtime::stub()).unwrap_or_else(
            |err| {
                panic!(
                    "Failed to read config file at {}: {:?}",
                    path.to_str().unwrap_or("<invalid path>"),
                    err
                )
            },
        )
    })
}

/// Helper function to get the compiled config as a byte array token stream, or None if no config path was provided
pub fn get_config_compiled_token_stream(
    config: Option<RuntimeConfig>,
) -> TokenStream {
    let config_bytes = config.map(compile_datex_config);

    config_bytes
        .map(|bytes| {
            quote! {
                Some(vec![#(#bytes),*])
            }
        })
        .unwrap_or_else(|| quote! { None })
}

/// Helper function to get the identifier and type of the argument at the given index, or return a syn::Error if it's not an identifier or if it's a receiver (self)
pub fn get_arg_ident_and_type(
    index: usize,
    func: &ItemFn,
    err_msg: &'static str,
) -> Result<(Ident, Box<Type>), syn::Error> {
    match func.sig.inputs.get(index).unwrap() {
        FnArg::Typed(pat_ty) => match &*pat_ty.pat {
            Pat::Ident(PatIdent { ident, .. }) => {
                Ok((ident.clone(), pat_ty.ty.clone()))
            }
            other => Err(syn::Error::new_spanned(other, err_msg)),
        },
        FnArg::Receiver(rcv) => Err(syn::Error::new_spanned(
            rcv,
            "Expected typed argument, not self",
        )),
    }
}

/// Compiles the given RuntimeConfig into DXB
fn compile_datex_config(config: RuntimeConfig) -> Vec<u8> {
    let CompilationResult {
        dxb_with_shared_values,
        ..
    } = compile_template(
        "?",
        vec![Some(ValueContainer::from(config))],
        CompileOptions::default(),
        // FIXME: stub runtime for now
        Runtime::stub(),
    )
    .expect("failed to compile DATEX config file");
    dxb_with_shared_values.dxb
}
