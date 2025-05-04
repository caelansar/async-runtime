use std::iter::FromIterator;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{Error, Expr, ExprLit, ItemFn, Lit, MetaNameValue, Result};

#[proc_macro_attribute]
pub fn main(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = TokenStream::from(item);
    let backup = item.clone();

    match async_block_on(attr.into(), item) {
        Ok(output) => output.into_token_stream().into(),
        Err(error) => TokenStream::from_iter([error.into_compile_error(), backup]).into(),
    }
}

#[proc_macro_attribute]
pub fn test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = TokenStream::from(item);
    let backup = item.clone();

    match test_internal(attr.into(), item) {
        Ok(output) => output.into_token_stream().into(),
        Err(error) => TokenStream::from_iter([error.into_compile_error(), backup]).into(),
    }
}

fn test_internal(attr: TokenStream, item: TokenStream) -> Result<ItemFn> {
    let mut item = async_block_on(attr, item)?;
    item.attrs.push(syn::parse_quote! { #[test] });

    Ok(item)
}

fn async_block_on(attr: TokenStream, item: TokenStream) -> Result<ItemFn> {
    let mut item: ItemFn = syn::parse2(item)?;

    let attr: MetaNameValue = syn::parse2(attr)?;

    if item.sig.asyncness.is_some() {
        item.sig.asyncness = None;
    } else {
        return Err(Error::new_spanned(item, "expected function to be async"));
    }

    let worker_threads = if attr.path.is_ident("worker_threads") {
        match attr.value {
            Expr::Lit(ExprLit {
                lit: Lit::Int(int), ..
            }) => int.to_string().parse::<usize>().ok(),
            _ => {
                return Err(Error::new_spanned(
                    attr.value,
                    "expected valid integer, e.g. 2",
                ));
            }
        }
    } else {
        None
    };

    let path = quote::quote! { ::cmoon };

    let span = item.span();
    let block = item.block;

    if let Some(worker_threads) = worker_threads {
        item.block = syn::parse_quote_spanned! {
            span =>
            {
                let mut executor = ::cmoon::init_with_threads(#worker_threads);
                executor.block_on(async {
                    #block
                })
            }
        };
    } else {
        item.block = syn::parse_quote_spanned! {
            span =>
            {
                #path::block_on(async {
                    #block
                })
            }
        };
    }

    Ok(item)
}
