use proc_macro::{TokenStream};
use proc_macro_error::{proc_macro_error, abort};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, GenericArgument, PathArguments, Type, Ident, Meta, NestedMeta, Lit, Attribute};

/// Returns Some<Type> if type is optional.
fn get_option_type(ty: &Type) -> Option<Type> {
    let syn::Type::Path(path_type) = ty else {return None};
    if path_type.path.segments.len() != 1 {
        return None;
    }
    let Some(option_segment) = path_type.path.segments.first() else {
        return None
    };
    if option_segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = option_segment.arguments.to_owned() else {return None};
    if arguments.args.len() != 1 {
        return None;
    }
    let first_argument = arguments.args.first();
    let Some(tu) = first_argument else {
        return None;
    };
    let GenericArgument::Type(result) = tu else { return None};
    Some(result.to_owned())
}

/// Returns Some<Type> if type is optional.
fn get_vec_type(ty: &Type) -> Option<Type> {
    let syn::Type::Path(path_type) = ty else {return None};
    if path_type.path.segments.len() != 1 {
        return None;
    }
    let Some(option_segment) = path_type.path.segments.first() else {
        return None
    };
    let PathArguments::AngleBracketed(arguments) = option_segment.arguments.to_owned() else {return None};
    if arguments.args.len() != 1 {
        return None;
    }
    let first_argument = arguments.args.first();
    let Some(tu) = first_argument else {
        return None;
    };
    let GenericArgument::Type(result) = tu else { return None};
    Some(result.to_owned())
}

fn get_one_at_a_time_ident(attribute: Attribute) -> Option<Ident> {
                       let meta = attribute.parse_meta().ok()?;
      let thing = match meta {
        Meta::List(list) => match list.nested.first() {
                Some(NestedMeta::Meta(Meta::NameValue(nv))) => {
                    let name = nv.path.get_ident().to_token_stream().to_string();
                    if name != "each" {
                        abort! { 
                            list,
                            "expected `builder(each = \"...\")`"
                        }
                    }

                    match &nv.lit {
                        Lit::Str(x) =>{
                            Some(x.value())
                        }
                        , _ => None
                    }
            }, _ => None
        },
        _ => None
    }?;
     let ident = format_ident!("{}", thing);
     Some(ident)
}

struct FieldInfo {
    ident:Ident,
    one_at_a_time_ident: Option<Ident>,
    // target_type: Type,
    is_optional: bool,
    builder_type: Type,
    only_one_at_a_time: bool,
}

#[proc_macro_error]
#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let builder_ident = format_ident!("{}Builder", ident);

    let Data::Struct(input_struct) = input.data else {panic!("Not a struct")};

    let members: Vec<_> = input_struct
        .fields
        .into_iter()
        .map(|field| {
            let ident = field.ident.expect("Failed to get identifier of field");
            // let ident = Some(base_ident);
            // let parsed_attrs = parse_meta!(field.attrs);
            // eprintln!("stuff {:?}", parsed_attrs);

            let one_at_a_time_ident = field.attrs.into_iter().find_map(get_one_at_a_time_ident);
            let only_one_at_a_time = one_at_a_time_ident.as_ref() == Some(&ident);
            // let name_string = field.attrs.into_iter().find_map(|attribute| attribute.tokens.into_iter().find_map(|tt| match tt {
            //     TokenTree::Literal(literal) => Some(literal.to_string()),
            //     _ => None,
            // }));
            // eprintln!("name {:?}", name_string);

            let target_type = field.ty;
            let option_type = get_option_type(&target_type);
            let is_optional = option_type.is_some();
            let builder_type = option_type.unwrap_or(target_type.clone());
            FieldInfo {ident, one_at_a_time_ident, is_optional, builder_type, only_one_at_a_time}
        })
        .collect();

    let builder_field_initializations = members
        .iter()
        .map(|FieldInfo {ident, one_at_a_time_ident,..}| match one_at_a_time_ident {
            None => quote!(#ident: std::option::Option::None),
            Some(_) => quote!(#ident: std::option::Option::Some(std::vec::Vec::new())),}
        );

    let builder_field_definitions = members
        .iter()
        .map(|FieldInfo {ident,builder_type,..}| quote!(#ident: std::option::Option<#builder_type>));

    let builder_methods = members.iter().filter_map(|FieldInfo {ident,builder_type,only_one_at_a_time,..}| {
        match only_one_at_a_time {
        false => Some(quote!(pub fn #ident(&mut self, #ident: #builder_type) -> &mut Self {
            self.#ident = std::option::Option::Some(#ident);
            self
        })),
        true => None
    }
    });

    let other_builder_methods = members.iter().filter_map(|FieldInfo {ident, one_at_a_time_ident,builder_type,..}| {
        let vec_type = get_vec_type(builder_type);
        match one_at_a_time_ident {
        Some(one_at_a_time_ident) => Some(quote!(pub fn #one_at_a_time_ident(&mut self, #one_at_a_time_ident: #vec_type) -> &mut Self {
            let vec = self.#ident.get_or_insert(std::vec::Vec::new());
            vec.push(#one_at_a_time_ident);
            self
        })),
        None => None
    }
    });

    let field_ready_checks = members.iter().map(|FieldInfo {ident,is_optional,..}| {
        match is_optional {
            true => quote!(let #ident = self.#ident.take();),
            false => quote!(let std::option::Option::Some(mut #ident) = self.#ident.take() else { 
                let error: std::boxed::Box<dyn std::error::Error> = std::string::String::from("Error").into();
                return std::result::Result::Err(error);
            };)
        }
        
    });

    let build_field_initializations = members.iter().map(|FieldInfo {ident,..}| quote!(#ident));

    let tokens = quote!(
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#builder_field_initializations),*
                }
            }
        }

        pub struct #builder_ident {
            #(#builder_field_definitions),*
        }

        impl #builder_ident {
            #(#builder_methods)*
            #(#other_builder_methods)*
            pub fn build(&mut self) -> std::result::Result<#ident, std::boxed::Box<dyn std::error::Error>> {
                // eprintln!("{:?}", self);
                #(#field_ready_checks)*
                std::result::Result::Ok(#ident {
                    #(#build_field_initializations),*
                })
            }
        }
    );

    // eprintln!("TOKENS: {}", tokens);

    return tokens.into();
}
