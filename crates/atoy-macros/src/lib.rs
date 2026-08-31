use std::{collections::HashMap, sync::Mutex};

use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{Expr, FnArg, Ident, ItemFn, LitStr, Pat::{self, Guard}, PatIdent, Path, ReturnType, Signature, Token, Type, parse::{Parse, ParseStream}, parse_macro_input, token::Mod};

static REGISTERED_FUNCTIONS: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);

fn get_registered_functions(mut callback: impl FnMut(&mut HashMap<String, Vec<String>>) -> ()) {
    let mut guard = REGISTERED_FUNCTIONS.lock().unwrap();
    let option: &mut Option<_> = &mut guard;
    if let Some(map) = option {
        callback(map);
    } else {
        let mut map = HashMap::new();
        callback(&mut map);
        *option = Some(map);
    }
}

struct ModPath {
    pub idents: Vec<Ident>
}

impl Parse for ModPath {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut idents = Vec::new();
        idents.push(input.parse()?);
        while !input.is_empty() {
            if input.parse::<Token![::]>().is_err() {
                break;
            }
            idents.push(input.parse()?);
        }
        Ok(Self { idents })
    }
}

fn path_to_string(path: &Path) -> String {
    path.segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<_>>().join("::")
}

struct TwoArgsInput(Path, Expr);

impl Parse for TwoArgsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let a: Path = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let b: Expr = input.parse()?;
        if !input.is_empty() {
            return Err(syn::Error::new(input.span(), "Expected 2 arguments"));
        }
        Ok(Self(a, b))
    }
}

#[proc_macro]
pub fn register_functions(input: TokenStream) -> TokenStream {
    let TwoArgsInput(path, instance) = parse_macro_input!(input as TwoArgsInput);
    let mut func_names = Vec::<String>::new();
    let mut shall_panic = false;
    get_registered_functions(|map| {
        if let Some(vector) = map.get(&path_to_string(&path)) {
            func_names = vector.clone();
        } else {
            shall_panic = true;
        }
    });
    // 拿出来panic避免毒化
    if shall_panic {
        panic!("No functions registered for path {:}", path_to_string(&path));
    }
    let idents = func_names.iter()
        .map(|s| syn::parse_str::<Ident>(s).expect(&format!("Illegal identifier {s}")));
    TokenStream::from(quote! {
        #(
            #path::#idents(#instance);
        )*
    })
}

#[proc_macro_attribute]
pub fn atoy_function(attr: TokenStream, item: TokenStream) -> TokenStream {
    let from = parse_macro_input!(attr as Path);
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
    get_registered_functions(|map| {
        if let Some(vector) = map.get_mut(&path_to_string(&from)) {
            vector.push(register_fn.to_string());
        } else {
            map.insert(path_to_string(&from), vec![register_fn.to_string()]);
        }
    });

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
                args.ensure_empty()?;
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
