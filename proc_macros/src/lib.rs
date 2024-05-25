extern crate proc_macro;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use darling::FromMeta;
use proc_macro::*;
use quote::{quote, ToTokens};

#[derive(Debug, FromMeta)]
struct Args {
    strategy: syn::Expr,
    output: syn::Expr,
    #[darling(default)]
    args_skip: String,
}

#[proc_macro_attribute]
pub fn skip_if(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = darling::ast::NestedMeta::parse_meta_list(args.into()).unwrap();
    let args = Args::from_list(&args).unwrap();
    let strategy = &args.strategy;
    let output = &args.output;
    let mut input: syn::ItemFn = syn::parse(input).unwrap();

    // Output type
    let syn::ReturnType::Type(_, output_type) = &input.sig.output else {
        panic!()
    };
    let syn::Type::Path(output_type) = output_type.as_ref() else {
        panic!();
    };

    let stms = &input.block.stmts;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (*stms).hash(&mut hasher);
    let code_hash = hasher.finish();

    let mut args_hash = vec![quote! {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
    }];
    let args_skip: HashSet<_> = args.args_skip.split(',').collect();
    for input in &input.sig.inputs {
        args_hash.push(match input {
            syn::FnArg::Receiver(_) => {
                quote! {self.hash(&mut hasher); }
            }
            syn::FnArg::Typed(tp) => {
                let syn::Pat::Ident(id) = &*tp.pat else {
                    panic!();
                };
                let pat = &tp.pat;
                if args_skip.contains(id.ident.to_string().as_str()) {
                    continue;
                }
                quote! { #pat.hash(&mut hasher); }
            }
        });
    }

    let res = if input.sig.asyncness.is_some() {
        quote! { (|| async {#(#stms)*})().await }
    } else {
        quote! { (|| {#(#stms)*})() }
    };

    input.block = Box::new(
        syn::parse2(quote! {{
            use skip_if::Strategy;
            use std::hash::{Hasher, Hash};

            let _args_hash = {
                #(#args_hash)*
                hasher.finish()
            };

            let skip_if_output = &#output;
            // Hack for type inference until `impl Trait` works for local variables
            // (see https://github.com/rust-lang/rust/issues/63066)
            fn get_strategy() -> impl Strategy<#output_type> {
               #strategy
            }
            let _strategy = get_strategy();
            // Call the strategy
            if _strategy.skip(skip_if_output, _args_hash, #code_hash) {
                tracing::warn!(?skip_if_output, "Skipping due to strategy");
                return Ok(());
            }
            // Use a closure to avoid early returns
            let res: #output_type = #res;
            // Callback
            if let Err(e) = _strategy.callback(&res, skip_if_output, _args_hash ,#code_hash) {
                tracing::warn!(?e, "Strategy callback failed");
            }
            Ok(res?)
        }})
        .unwrap(),
    );

    input.into_token_stream().into()
}
