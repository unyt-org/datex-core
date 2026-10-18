use proc_macro2::TokenStream;
use quote::quote;
use syn::{ExprClosure, ImplItemFn, ItemFn, Signature, Type, fold, fold::Fold};

pub fn generate_native_callable_from_impl_fn(
    function: &ImplItemFn,
    self_type: &Type,
) -> TokenStream {
    generate_native_callable(
        function.sig.asyncness.is_some(),
        &function.sig,
        Some(self_type),
    )
}

pub fn generate_native_callable_from_fn(function: &ItemFn) -> TokenStream {
    generate_native_callable(
        function.sig.asyncness.is_some(),
        &function.sig,
        None,
    )
}

// TODO: use for inline macro native_callable!( |args| ... )
pub fn generate_native_callable_from_closure(
    _function: &ExprClosure,
) -> TokenStream {
    todo!()
    // generate_native_callable(
    //     function.asyncness.is_some(),
    // )
}

/// Generates the code that creates a [Callable] from a given rust function signature and optional self type.
pub fn generate_native_callable(
    is_async: bool,
    sig: &Signature,
    self_ty: Option<&Type>,
) -> TokenStream {
    let method_ident = &sig.ident;
    let method_name = method_ident.to_string();

    let return_type = match &sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, Type::Tuple(tuple))
            if tuple.elems.is_empty() =>
        {
            None
        }
        syn::ReturnType::Type(_, ty) => Some(ty),
    };
    // if return type is Result<T, E>, extract T and E for yeet_type and return_type
    let (return_type, yeet_type) = extract_return_and_yeet_type(return_type);

    let return_type = return_type.map(|ty| replace_self_type(&ty, self_ty));
    let yeet_type = yeet_type.map(|ty| replace_self_type(&ty, self_ty));

    let return_type_tokens = match return_type {
        Some(ref ty) => {
            quote! { Some(Box::new(<#ty as GetDatexType>::datex_type(cache))) }
        }
        None => quote! { None },
    };
    let yeet_type_tokens = match yeet_type {
        Some(ref ty) => {
            quote! { Some(Box::new(<#ty as GetDatexType>::datex_type(cache))) }
        }
        None => quote! { None },
    };

    let mut parameter_defs = Vec::new();

    for param in &sig.inputs {
        match param {
            syn::FnArg::Receiver(_ty) => {
                let ty =
                    self_ty.expect("Self type must be provided for methods");
                parameter_defs.push(quote! {
                    (
                        Some("self".to_string()),
                        <#ty as GetDatexType>::datex_type(cache)
                    )
                });
            }
            syn::FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    syn::Pat::Ident(ident) => ident.ident.to_string(),
                    _ => {
                        panic!("Unsupported parameter pattern")
                    }
                };
                // normalize type name (replace Self with the actual type name)
                let ty = replace_self_type(&pat_type.ty, self_ty);
                parameter_defs.push(quote! {
                    (
                        Some(#name.to_string()),
                        <#ty as GetDatexType>::datex_type(cache)
                    )
                });
            }
        }
    }

    let has_mutable_inputs = sig.inputs.iter().any(|param| match param {
        syn::FnArg::Receiver(receiver) => {
            receiver.mutability.is_some() || matches!(receiver.kind, syn::ReceiverKind::Reference(_, _, Some(_)))
        }
        syn::FnArg::Typed(pat_type) => {
            if let Type::Reference(type_reference) = &*pat_type.ty {
                type_reference.mutability.is_some()
            } else {
                false
            }
        }
    });

    let mut call_argument_inits = Vec::new();
    let mut call_argument_accesses = Vec::new();
    let mut call_arguments = Vec::new();
    let mut call_argument_collections = Vec::new();

    for (index, param) in sig.inputs.iter().enumerate() {
        let ty = match param {
            syn::FnArg::Receiver(_receiver) => self_ty.unwrap().clone(),
            syn::FnArg::Typed(pat_type) => {
                replace_self_type(&pat_type.ty, self_ty)
            }
        };
        let var_ident_container = syn::Ident::new(
            &format!("arg_container_{}", index),
            proc_macro2::Span::call_site(),
        );
        let var_ident = syn::Ident::new(
            &format!("arg_{}", index),
            proc_macro2::Span::call_site(),
        );
        let is_borrowed = match param {
            syn::FnArg::Receiver(receiver) => {
                matches!(receiver.kind, syn::ReceiverKind::Reference(..))
            }
            syn::FnArg::Typed(pat_type) => {
                matches!(&*pat_type.ty, Type::Reference(..))
            }
        };
        call_argument_inits.push(quote! {
            let mut #var_ident_container = val_iter.next().unwrap();
        });

        // distinguish between move, & and &mut
        call_argument_accesses.push(
            // handle & and &mut
            if is_borrowed {
                quote! {
                    let mut value_sheep = (&mut #var_ident_container.value).value_container_mut(); // collapse potential Shared to inner ValueContainer
                    // try to get stored native value from the value container
                    let #var_ident = if let Ok(mut inner) = <#ty as ConvertValueContainer>::try_borrow_mut_from_value_container(core::ops::DerefMut::deref_mut(&mut value_sheep)) {
                        inner
                    } else {
                        // fallback: handle serde conversion
                        todo!()
                        //&mut (<#ty as #datex_core_crate_name::datex_proxy::DatexValueContainerProxyDeserialize>::try_from_value_container(value_sheep.clone()).unwrap())
                    };
                }
            }
            // handle move
            else {
                quote! {
                    // try to get stored native value from the value container
                    let #var_ident = match <#ty as ConvertValueContainer>::try_from_value_container(#var_ident_container.value) {
                        Ok(inner) => inner,
                        Err(value) => {
                            // fallback: handle serde conversion
                            todo!()
                        }
                    };
                }
            }
        );
        call_arguments.push(quote! { #var_ident });

        call_argument_collections.push(
            // borrowed values are collected and returned back
            if is_borrowed {
                quote! { if #var_ident_container.passed_as_ref { Some(#var_ident_container.value) } else { None } }
            }
            // moved values can no longer be accessed, so we return None
            else {
                quote! { None }
            }
        );
    }

    // reverse the call_arguments initialization, but keep usage in the original order
    call_argument_accesses.reverse();

    let method_path = if let Some(self_ty) = self_ty {
        quote! { #self_ty::#method_ident }
    } else {
        quote! { #method_ident }
    };

    let original_method_call = if is_async {
        quote! { #method_path(#(#call_arguments),*).await }
    } else {
        quote! { #method_path(#(#call_arguments),*) }
    };

    let method_call_body = quote! {{
        #(#call_argument_accesses)*
        #original_method_call
    }};

    // with return type, wrap in Some, otherwise return None
    let method_call_conversion = if return_type.is_some() {
        // Note: since the borrowed cache is no longer accessible inside the function body,
        // the return type is fetched during creation and stored in the closure context.
        quote! {{
            let mut val_iter = vals.into_iter();

            #(#call_argument_inits)*
            
            let mut result_value = ConvertValueContainer::to_value_container(
                #method_call_body,
            );

            (Some(result_value), vec![#(#call_argument_collections),*].into_iter().filter_map(|v| v).collect())
        }}
    } else {
        quote! {{
            let mut val_iter = vals.into_iter();

            #(#call_argument_inits)*

            #method_call_body;
            (None, vec![#(#call_argument_collections),*].into_iter().filter_map(|v| v).collect())
        }}
    };

    let kind = if has_mutable_inputs {
        quote! {  CallableKind::Procedure }
    } else {
        quote! {  CallableKind::Function }
    };

    let body = if is_async {
        quote! {
            CallableBody::native_async(move |mut vals, runtime| {
                let runtime = runtime.clone();
                alloc::boxed::Box::pin(async move {Ok(#method_call_conversion)})
            })
        }
    } else {
        quote! {
            CallableBody::native_sync(move |mut vals, runtime| {
                let runtime = runtime.clone();
                Ok(#method_call_conversion)
            })
        }
    };

    quote! {{

        Callable {
            name: Some(#method_name.to_string()),
            signature: CallableTypeDefinition {
                kind: #kind,
                requires_async: #is_async,
                parameters: vec![#(#parameter_defs),*],
                rest_parameter: None,
                return_type: #return_type_tokens,
                yeet_type: #yeet_type_tokens,
            },
            body: #body,
            creator: Default::default(),
        }
    }}
}

/// A helper struct to replace occurrences of `Self` in a type with a specific type.
/// TODO: Move to utils
struct SelfTypeReplacer<'a> {
    self_ty: &'a syn::Type,
}

impl Fold for SelfTypeReplacer<'_> {
    fn fold_type(&mut self, ty: syn::Type) -> syn::Type {
        match &ty {
            syn::Type::Path(path)
                if path.qself.is_none() && path.path.is_ident("Self") =>
            {
                self.self_ty.clone()
            }
            _ => fold::fold_type(self, ty),
        }
    }
}
/// A helper function to replace occurrences of `Self` in a type with a specific type.
fn replace_self_type(ty: &syn::Type, self_ty: Option<&syn::Type>) -> syn::Type {
    match self_ty {
        Some(self_ty) => SelfTypeReplacer { self_ty }.fold_type(ty.clone()),
        None => ty.clone(),
    }
}

fn extract_return_and_yeet_type(
    return_type: Option<&Box<Type>>,
) -> (Option<Type>, Option<Type>) {
    if let Some(ty) = return_type {
        if let syn::Type::Path(type_path) = &**ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Result" {
                    if let syn::PathArguments::AngleBracketed(args) =
                        &segment.arguments
                    {
                        let mut args_iter = args.args.iter();
                        let ok_type = args_iter.next().and_then(|arg| {
                            if let syn::GenericArgument::Type(ty) = arg {
                                Some(ty)
                            } else {
                                None
                            }
                        });
                        let err_type = args_iter.next().and_then(|arg| {
                            if let syn::GenericArgument::Type(ty) = arg {
                                Some(ty)
                            } else {
                                None
                            }
                        });
                        (ok_type.cloned(), err_type.cloned())
                    } else {
                        (Some(*ty.clone()), None)
                    }
                } else {
                    (Some(*ty.clone()), None)
                }
            } else {
                (Some(*ty.clone()), None)
            }
        } else {
            (Some(*ty.clone()), None)
        }
    } else {
        (None, None)
    }
}
