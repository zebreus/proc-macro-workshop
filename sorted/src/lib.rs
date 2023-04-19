use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

fn sorted_helper(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, syn::Error> {
    let _ = args;
    let input = syn::parse::<ItemEnum>(input)?;
    // let backstream: TokenStream = input.;

    let variants = input.clone().variants;
    // let variants = input.variants.iter().map(|variant| ).collect_vec();

    let _last_name = variants.iter().try_fold(String::from("aaaaaaaa"), |previous, variant| {
        let name = variant.ident.to_string();
        if name.to_lowercase() < previous.to_lowercase() {
            let before = variants.iter().map(|variant| variant.ident.to_string()).find(|variant| variant.to_lowercase() > name.to_lowercase()).unwrap();
            Err(syn::Error::new(variant.ident.span(), format!("{} should sort before {}",name, before)))
        }else {
            Ok(name)
        }
    })?;

    let token_stream = quote!(#input
    
    struct Toaster {});

    return Ok(token_stream.into());
}

#[proc_macro_attribute]
pub fn sorted(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let result = sorted_helper(args, input.clone());

    match result {
        Ok(stream) => stream,
        Err(error) => {
            let compile_error = error.into_compile_error();
            // compile_error.into()
            let original_stream: TokenStream = input.into();
            quote!(
                #compile_error
                #original_stream
            ).into()
        }
    }
}
