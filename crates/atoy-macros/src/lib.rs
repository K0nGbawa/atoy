use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Expr, FnArg, Ident, ItemFn, Pat, PatIdent, Path, ReturnType, Signature, Token, Type, parenthesized, parse::{Parse, ParseStream}, parse_macro_input, punctuated::Punctuated};

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
    
    // if inputs.is_empty() {
    //     let expanded = quote! {
    //         #(#attrs)*
    //         #vis #sig #block

    //         pub fn #register_fn(vm: &mut crate::vm::VM) {
    //             vm.register_func(stringify!(#name), ::std::rc::Rc::new(|args: crate::vm::Args| -> Result<crate::parser::Value> {
    //                 let result = #name();
    //                 Ok(result.into())
    //             }))
    //         }
    //     };
    //     return TokenStream::from(expanded);
    // }
    let param_names: Vec<_> = params.iter().map(|p| &p.name).collect();
    let len = param_names.len();
    let mut found_args = false;
    let param_errors: Vec<_> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let arg_name = &p.name;
            let ty = &p.ty;
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if seg.ident == "Args" && i == params.len() - 1 {
                        found_args = true;
                        return quote_spanned! { arg_name.span() =>
                            let #arg_name = args;
                        }
                    } else if seg.ident == "Args" {
                        return quote! {
                            compile_error!("The Args must be the last parameter.");
                        }
                    }
                }
            }
            quote_spanned! { arg_name.span() =>
                let #arg_name = args.get_arg::<#ty>(#i)
                    .map_err(|e| match e {
                        crate::vm::RuntimeError::TypeError {
                            expected,
                            found,
                            thrower: None
                        } => crate::vm::RuntimeError::TypeError {
                            expected,
                            found,
                            thrower: Some(concat!("Built-in Function `", stringify!(#name), "()`"))
                        },
                        other => other
                    })?;
            }
        })
        .collect();

    let ensure_length = if !found_args {
        quote! {
            args.ensure_len(#len)?;
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let invoke = match output {
        ReturnType::Default => {
            quote! {
                #name(#(#param_names),*);
                Ok(crate::parser::Value::None)
            }
        }
        ReturnType::Type(_, _) => {
            quote! {
                let ret = #name(#(#param_names),*);
                Ok(crate::parser::Value::from(ret))
            }
        }
    };

    let expanded = quote! {
        #(#attrs)*
        #vis #sig #block

        pub fn #register_fn(vm: &mut crate::vm::VM) {
            vm.register_func(stringify!(#name), ::std::rc::Rc::new(|args: crate::vm::Args| -> crate::vm::RuntimeResult<crate::parser::Value> {
                #ensure_length
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

struct RegisterFnArgs {
    vm_expr: Expr,
    funcs: Punctuated<Path, Token![,]>,
}

impl Parse for RegisterFnArgs {
    // TODO: implement parsing logic
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vm_expr = input.parse()?;
        let _ = input.parse::<Token![,]>()?;
        let content;
        let _paren = parenthesized!(content in input);
        let funcs = Punctuated::<Path, Token![,]>::parse_terminated(&content)?;
        Ok(Self { vm_expr, funcs })
    }
}

#[proc_macro]
pub fn register_fns(input: TokenStream) -> TokenStream {
    let RegisterFnArgs { vm_expr, funcs } = parse_macro_input!(input as RegisterFnArgs);
    let (paths, funcs): (Vec<Path>, Vec<Ident>) = funcs.into_iter()
        .map(|p| {
            let mut punc = p.segments.clone();
            let last = punc.pop().expect("should be at least one segment");
            let ident = last.ident;
            (Path { segments: punc, leading_colon: None}, format_ident!("__atoy_register_{}", ident))
        })
        .unzip();
    let expanded = quote! {
        #(
            #paths #funcs(#vm_expr);
        )*
    };
    TokenStream::from(expanded)
}