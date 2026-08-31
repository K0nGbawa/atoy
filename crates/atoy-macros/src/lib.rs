use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{FnArg, Ident, ItemFn, Pat, PatIdent, ReturnType, Signature, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn atoy_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(item as ItemFn);
    let sig = &ast.sig;
    let vis = &ast.vis;
    let block = &ast.block;
    let attrs = &ast.attrs;
    let name = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let params = collect_params(sig);
    let register_fn = format_ident!("__atoy_register_{}", name);

    if inputs.is_empty() {
        let expanded = quote! {
            #(#attrs)*
            #vis #sig #block

            pub fn #register_fn(vm: &mut crate::vm::VM) {
                vm.register_func(stringify!(#name), ::std::rc::Rc::new(|args: crate::vm::Args| -> Result<crate::parser::Value> {
                    let result = #name();
                    Ok(result.into())
                }))
            }
        };
        return TokenStream::from(expanded);
    }
    let param_names: Vec<_> = params.iter().map(|p| &p.name).collect();
    let param_errors: Vec<_> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = &p.name;
            let ty = &p.ty;
            quote_spanned! { name.span() =>
                let #name = args.take::<#ty>().map_err(|e| format!("处理第 {} 个时参数出错: {}", #i + 1, e))?;
            }
        })
        .collect();

    let invoke = match output {
        ReturnType::Default => {
            quote! {
                #name(#(#param_names),*);
                Ok(crate::parser::Value)
            }
        }
        ReturnType::Type(_, _) => {
            quote! {
                let ret = #name(#(#param_names),*);
                Ok(crate::vm::IntoAtoyValue::into_value(ret))
            }
        }
    };

    let expanded = quote! {
        #(#attrs)*
        #vis #sig #block

        pub fn #register_fn(vm: &mut crate::vm::VM) {
            vm.register_func(stringify!(#name), ::std::rc::Rc::new(|mut args: crate::vm::Args| -> Result<crate::parser::Value, String> {
                #(#param_errors)*
                #invoke
            }));
        }
    };
    TokenStream::from(expanded)
}

struct ParamInfo {
    name: Ident,
    ty: Type,
}

fn collect_params(sig: &Signature) -> Vec<ParamInfo> {
    let mut params = Vec::new();
    for input in &sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(PatIdent { ident, .. }) = *pat_type.pat.clone() {
                let ty = (*pat_type.ty).clone();
                params.push(ParamInfo { name: ident, ty });
            }
        }
    }
    params
}
