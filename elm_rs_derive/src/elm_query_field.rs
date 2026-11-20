#![allow(unused_variables)]
//! Derive macro for ElmQuery.

use crate::{EnumVariantKind, Intermediate, TypeInfo};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};
use tracing::instrument;

#[instrument(skip(input))]
pub(crate) fn derive(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let intermediate = match Intermediate::parse(derive_input) {
        Ok(intermediate) => intermediate,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let token_stream = match intermediate_to_token_stream(intermediate) {
        Ok(token_stream) => token_stream,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    TokenStream::from(token_stream)
}

fn intermediate_to_token_stream(
    Intermediate {
        ident,
        elm_type,
        mut generics,
        generics_without_bounds,
        type_info,
        container_attributes,
    }: Intermediate,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let query_field_encoder_name = format!("queryFieldEncoder{elm_type}");
    let query_field_decoder_name = format!("queryFieldDecoder{elm_type}");
    let (ts, decode) = match type_info {
        TypeInfo::Enum { variants, .. } => {
            let mut branches = Vec::new();
            let mut arms = Vec::new();
            for variant in variants {
                #[cfg(feature = "serde")]
                if variant.serde_attributes.skip {
                    continue;
                }
                if let EnumVariantKind::Unit = variant.variant {
                    let elm_name = variant.name_elm();
                    let name_encode = variant.name_encode();
                    branches.push(format!("{elm_name} -> \"{name_encode}\""));
                    let mut texts = vec![name_encode];
                    #[cfg(feature = "serde")]
                    {
                        for alias in variant.decode_names() {
                            if !texts.contains(&alias) {
                                texts.push(alias);
                            }
                        }
                    }
                    for text in texts {
                        arms.push(format!("\"{text}\" -> Ok {elm_name}"));
                    }
                } else {
                    return Err(syn::Error::new(
                        variant.span,
                        "only unit variants are allowed",
                    ));
                }
            }
            arms.push("unexpected -> Err (\"unexpected \" ++ unexpected)".to_string());

            let encode = quote! {::std::format!("\
{function_name} : {elm_type} -> String
{function_name} var =
    case var of
        {branches}
",
                function_name = #query_field_encoder_name,
                elm_type = #elm_type,
                branches = (
                    &[
                        #(#branches),*
                    ]
                ).join("\n        ")
            )};
            let decode = quote! {::std::format!("\
{function_name} : String -> Result String {elm_type}
{function_name} text =
    case text of
        {arms}
",
                function_name = #query_field_decoder_name,
                elm_type = #elm_type,
                arms = (
                    &[
                        #(#arms),*
                    ]
                ).join("\n        ")
            )};
            (encode, decode)
        }
        _ => return Err(syn::Error::new(ident.span(), "only enums are allowed")),
    };

    for p in generics.type_params_mut() {
        p.bounds.push(syn::parse_str("::rs2elm::Elm").unwrap());
        p.bounds
            .push(syn::parse_str("::rs2elm::ElmQueryField").unwrap());
    }

    let res = quote! {
        impl #generics ::rs2elm::ElmQueryField for #ident #generics_without_bounds {
            fn query_field_type() -> &'static str {
                "Url.Builder.string"
            }

            fn query_field_encoder_name() -> &'static str {
                #query_field_encoder_name
            }

            fn query_field_encoder_definition() -> ::std::option::Option<::std::string::String> {
                ::std::option::Option::Some(#ts)
            }

            fn query_field_form_value() -> &'static str {
                #query_field_decoder_name
            }

            fn query_field_decoder_definition() -> ::std::option::Option<::std::string::String> {
                ::std::option::Option::Some(#decode)
            }

            fn contribute_query_field(module: &mut ::rs2elm::export::Module) {
                <Self as ::rs2elm::Elm>::contribute(module);
                if let ::std::option::Option::Some(definition) = <Self as ::rs2elm::ElmQueryField>::query_field_encoder_definition() {
                    if module.mark(<Self as ::rs2elm::ElmQueryField>::query_field_encoder_name()) {
                        module.push_definition(&definition);
                    }
                }
                if let ::std::option::Option::Some(definition) = <Self as ::rs2elm::ElmQueryField>::query_field_decoder_definition() {
                    if module.mark(#query_field_decoder_name) {
                        module.push_definition(&definition);
                    }
                }
            }
        }
    };
    Ok(res)
}
