use crate::utils::expr_to_value_container;
use datex_core::{
    self,
    compiler::{CompileOptions, compile_template},
    core_compiler::core_compilation_context::DXBWithSharedValues,
    prelude::*,
    runtime::Runtime,
    values::value_container::ValueContainer,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

enum Placeholder {
    ValueContainer(ValueContainer),
    Expression(Expr),
}

pub struct ExecuteMacroInput {
    program: String,
    args: Vec<Placeholder>,
}

impl Parse for ExecuteMacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let program: LitStr = input.parse()?;
        let mut tokened_args = Punctuated::<Expr, Token![,]>::new();

        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            while !input.is_empty() {
                tokened_args.push_value(input.parse()?);
                if input.peek(Token![,]) {
                    tokened_args.push_punct(input.parse()?);
                } else {
                    break;
                }
            }
        }
        let mut args = Vec::new();
        for arg in tokened_args.into_iter() {
            if let Ok(value) = expr_to_value_container(&arg) {
                args.push(Placeholder::ValueContainer(value));
            } else {
                args.push(Placeholder::Expression(arg));
            }
        }

        Ok(Self {
            program: program.value(),
            args,
        })
    }
}

fn prepare_setup(input: ExecuteMacroInput) -> TokenStream {
    let runtime = Runtime::stub();
    let script = input.program;

    let placeholder_count = script.chars().filter(|&c| c == '?').count();
    let arg_count = input.args.len();

    if placeholder_count != arg_count {
        return syn::Error::new_spanned(
            script,
            format!(
                "execute!: placeholder count ({}) != argument count ({})",
                placeholder_count, arg_count
            ),
        )
        .to_compile_error();
    }

    let stack_init = input
        .args
        .iter()
        .map(|placeholder| match placeholder {
            Placeholder::ValueContainer(_) => quote! {
                stack_values.push(None);
            },
            Placeholder::Expression(expr) => quote! {
                stack_values.push(Some(ValueContainer::from(#expr)));
            },
        })
        .collect::<Vec<_>>();

    let placeholders: Vec<Option<ValueContainer>> = input
        .args
        .into_iter()
        .map(|p| match p {
            Placeholder::ValueContainer(v) => Some(v),
            Placeholder::Expression(_) => None,
        })
        .collect::<Vec<_>>();

    let dxb = compile_template(
        &script,
        placeholders,
        CompileOptions::default(),
        runtime,
    );
    if let Err(e) = dxb {
        return syn::Error::new_spanned(
            script,
            format!("Failed to compile template: {}", e),
        )
        .to_compile_error();
    }
    let DXBWithSharedValues { dxb, shared_values } = dxb.unwrap().dxb_with_shared_values;

    if dxb.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "execute!: compiled DXB body is empty for script `{}`",
                script
            ),
        )
        .to_compile_error();
    }
    println!("compiled DXB length = {}", dxb.len());
    println!("shared values length = {}", shared_values.len());

    quote! {{
        use datex_core::runtime::execution::execution_loop::state::RuntimeExecutionStack;
        use datex_core::runtime::execution::execution_input::ExecutionCallerMetadata;
        use datex_core::values::value_container::ValueContainer;
        use datex_core::collections::HashMap;
        use datex_core::runtime::execution::{ExecutionInput, ExecutionOptions};
        use datex_core::runtime::Runtime;
        use datex_core::prelude::*;
        use datex_core::core_compiler::core_compilation_context::DXBWithSharedValues;

        let mut stack_values: Vec<Option<ValueContainer>> = Vec::new();
        #(#stack_init)*

        let runtime_execution_stack = RuntimeExecutionStack { values: stack_values };
        let dxb_with_shared_values = DXBWithSharedValues {
            dxb: vec![#(#dxb),*],
            shared_values: vec![],
        };

        let runtime = Runtime::stub();
        ExecutionInput::new_with_stack(
            dxb_with_shared_values,
            ExecutionCallerMetadata::local_default(),
            ExecutionOptions::default(),
            runtime,
            runtime_execution_stack
        )
    }}
}

pub fn execute_sync(input: ExecuteMacroInput) -> TokenStream {
    let setup = prepare_setup(input);
    quote! {{
        datex_core::runtime::execution::execute_dxb_sync(#setup)
    }}
}
pub fn execute_async(input: ExecuteMacroInput) -> TokenStream {
    let setup = prepare_setup(input);
    quote! {{
        async move {
            datex_core::runtime::execution::execute_dxb(#setup).await
        }
    }}
}
pub fn execute_sync_unchecked(input: ExecuteMacroInput) -> TokenStream {
    let setup = prepare_setup(input);
    quote! {{
        datex_core::runtime::execution::execute_dxb_sync(#setup).expect("Failed to execute DXB")
    }}
}
pub fn execute_async_unchecked(input: ExecuteMacroInput) -> TokenStream {
    let setup = prepare_setup(input);
    quote! {{
        async move {
            datex_core::runtime::execution::execute_dxb(#setup).await.expect("Failed to execute DXB")
        }
    }}
}
