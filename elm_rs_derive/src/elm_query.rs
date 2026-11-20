#![allow(unused_variables)]
//! Derive macro for ElmQuery.

use super::polymorph::{self, Forms};
use crate::{Intermediate, TypeInfo};
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
    let forms = Forms::from_generics(&elm_type, &generics);
    let param_idents: Vec<syn::Ident> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();
    let (ts, form, contributes) = match type_info {
        TypeInfo::Struct(fields) => {
            let outer = &forms.query_prefix;
            let binders = &forms.query_binders;
            let helper = if forms.params.is_empty() {
                format!("String -> {}", forms.applied)
            } else {
                let arrows: String = forms
                    .params
                    .iter()
                    .map(|param| format!("({} -> String) -> ", param.var))
                    .collect();
                format!("{arrows}String -> {}", forms.applied)
            };
            let recursive = fields.iter().any(|field| field.lazy);
            let pattern = if recursive {
                format!("({elm_type} struct)")
            } else {
                "struct".to_string()
            };
            let mut pieces = Vec::new();
            let mut parsers = Vec::new();
            let mut contributes = Vec::new();
            let mut decode_names = Vec::new();
            for field in &fields {
                let key = format!("(fieldKey \"{}\")", field.name_encode());
                let value = format!("struct.{}", field.name_elm());
                let emitted =
                    polymorph::query_emit(&field.ty, &key, &value, &forms.params, field.index)?;
                pieces.push(emitted.list);
                parsers.push(emitted.parser);
                contributes.push(emitted.contribute);
                decode_names.push(field.name_encode());
            }
            let ctor = form_constructor(&elm_type, &fields);
            let form_binders = form_binders(&forms);
            let form_arrows = form_arrows(&forms);
            let form_result = if forms.applied.contains(' ') {
                format!("Result String ({})", forms.applied)
            } else {
                format!("Result String {}", forms.applied)
            };
            // The helper comes first. `elm repl` compiles each declaration on its own
            // and rejects a forward reference from the wrapper.
            let template = "\
elmRsQuery{elm_type} : {helper} -> List Url.Builder.QueryParameter
elmRsQuery{elm_type} {binders}prefix {pattern} =
    let
        fieldKey name =
            if prefix == \"\" then
                name
            else
                prefix ++ \"[\" ++ name ++ \"]\"
    in
    List.concat
        [ {fields}
        ]

urlEncode{elm_type} : {outer} -> List Url.Builder.QueryParameter
urlEncode{elm_type} {binders}value =
    elmRsQuery{elm_type} {binders}\"\" value
";
            let form_template = "\
elmRsDecode{elm_type} : {form_sig}
elmRsDecode{elm_type} {form_binders}prefix pairs =
    let
        fieldKey name =
            if prefix == \"\" then
                name
            else
                prefix ++ \"[\" ++ name ++ \"]\"
    in
    Ok {ctor}
        {steps}

urlDecode{elm_type} : {url_sig}
urlDecode{elm_type} {form_binders}pairs =
    elmRsDecode{elm_type} {form_binders}\"\" pairs
";
            let ts = quote! {
                ::std::format!(
                    #template,
                    elm_type = #elm_type,
                    outer = #outer,
                    binders = #binders,
                    helper = #helper,
                    pattern = #pattern,
                    fields = {
                        let parts: &[::std::string::String] = &[#(#pieces),*];
                        parts.join("\n        , ")
                    },
                )
            };
            let form_sig = format!("{form_arrows}String -> List (String, String) -> {form_result}");
            let url_sig = format!("{form_arrows}List (String, String) -> {form_result}");
            let form = quote! {
                ::std::format!(
                    #form_template,
                    elm_type = #elm_type,
                    form_sig = #form_sig,
                    url_sig = #url_sig,
                    form_binders = #form_binders,
                    ctor = #ctor,
                    steps = {
                        let parsers: &[::std::string::String] = &[#(#parsers),*];
                        let names = [#(#decode_names),*];
                        let mut steps = ::std::vec::Vec::new();
                        let mut index = 0usize;
                        while index < parsers.len() {
                            steps.push(::std::format!(
                                "|> Result.andThen (\\build -> Result.map build ({parser} (fieldKey \"{name}\") pairs))",
                                parser = parsers[index],
                                name = names[index],
                            ));
                            index += 1;
                        }
                        steps.join("\n        ")
                    },
                )
            };
            (ts, form, contributes)
        }
        _ => {
            return Err(syn::Error::new(
                ident.span(),
                "only structs with named fields are allowed",
            ));
        }
    };
    let nested_args = if param_idents.is_empty() {
        quote! { ::std::string::String::new() }
    } else {
        quote! {
            {
                let parts = [
                    #( {
                        // A generic query always writes `Url.Builder.string` and expects
                        // `(a -> String)`. An integer field's own encoder is `identity`
                        // (`Int -> Int`), which does not typecheck in that argument.
                        let field_type = <#param_idents as ::rs2elm::ElmQueryField>::query_field_type();
                        if field_type == "Url.Builder.int" {
                            ::std::string::String::from("String.fromInt")
                        } else {
                            ::std::string::String::from(<#param_idents as ::rs2elm::ElmQueryField>::query_field_encoder_name())
                        }
                    } ),*
                ];
                ::std::format!("{} ", parts.join(" "))
            }
        }
    };
    let form_args = if param_idents.is_empty() {
        quote! { ::std::string::String::new() }
    } else {
        quote! {
            {
                let parts = [
                    #( ::std::string::String::from(<#param_idents as ::rs2elm::ElmQueryField>::query_field_form_value()) ),*
                ];
                ::std::format!(" {}", parts.join(" "))
            }
        }
    };

    for p in generics.type_params_mut() {
        p.bounds.push(syn::parse_str("::rs2elm::Elm").unwrap());
        p.bounds
            .push(syn::parse_str("::rs2elm::ElmQueryField").unwrap());
    }

    let res = quote! {
        impl #generics ::rs2elm::ElmQuery for #ident #generics_without_bounds {
            fn elm_query() -> ::std::string::String {
                #ts
            }

            fn elm_form() -> ::std::string::String {
                #form
            }

            fn query_nested(prefix: &str, value: &str) -> ::std::string::String {
                let args = #nested_args;
                ::std::format!(
                    "elmRsQuery{elm} {args}{prefix} {value}",
                    elm = <Self as ::rs2elm::Elm>::elm_type(),
                    args = args,
                    prefix = prefix,
                    value = value,
                )
            }

            fn form_parser() -> ::std::string::String {
                let args = #form_args;
                let elm = <Self as ::rs2elm::Elm>::elm_type();
                if args.is_empty() {
                    ::std::format!("elmRsDecode{elm}")
                } else {
                    ::std::format!("(elmRsDecode{elm}{args})")
                }
            }

            fn contribute_query(module: &mut ::rs2elm::export::Module) {
                <Self as ::rs2elm::Elm>::contribute(module);
                if module.mark("elmRsFormFind") {
                    module.push_definition(::rs2elm::form_helpers());
                }
                let name = ::std::format!("urlEncode{}", <Self as ::rs2elm::Elm>::elm_type());
                if !module.mark(&name) {
                    return;
                }
                #(#contributes)*
                module.push_definition(&<Self as ::rs2elm::ElmQuery>::elm_query());
                module.push_definition(&<Self as ::rs2elm::ElmQuery>::elm_form());
            }
        }
    };
    Ok(res)
}

fn form_binders(forms: &Forms) -> String {
    if forms.params.is_empty() {
        String::new()
    } else {
        format!(
            "{} ",
            forms
                .params
                .iter()
                .map(|param| format!("{}Form", param.var))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

fn form_arrows(forms: &Forms) -> String {
    forms
        .params
        .iter()
        .map(|param| format!("(String -> Result String {}) -> ", param.var))
        .collect()
}

/// A recursive struct is one Elm constructor around a record.
/// A type alias constructs from the fields in order.
fn form_constructor(elm_type: &str, fields: &[crate::StructField]) -> String {
    if !fields.iter().any(|field| field.lazy) {
        return elm_type.to_string();
    }
    let binds = fields
        .iter()
        .map(|field| field.name_elm())
        .collect::<Vec<_>>()
        .join(" ");
    let record = fields
        .iter()
        .map(|field| {
            let name = field.name_elm();
            format!("{name} = {name}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("(\\{binds} -> {elm_type} {{ {record} }})")
}
