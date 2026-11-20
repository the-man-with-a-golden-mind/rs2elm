#![allow(unused_variables)]
//! Derive macro for ElmDecode.

use super::{EnumVariantKind, Intermediate, TypeInfo, polymorph::Forms};
#[cfg(feature = "serde")]
use crate::attributes::serde::EnumRepresentation;
use crate::{EnumVariant, StructField, attributes::ContainerAttributes};
use heck::ToLowerCamelCase;
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
        Err(err) => return err.to_compile_error().into(),
    };
    let token_stream = match intermediate_to_token_stream(intermediate) {
        Ok(token_stream) => token_stream,
        Err(err) => return err.to_compile_error().into(),
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
    let decoder_type = format!("{}Decoder", elm_type.to_lower_camel_case());
    let forms = Forms::from_generics(&elm_type, &generics);
    let contribute_fields = super::contribute_calls(
        &super::dependency_types(&type_info),
        quote! { ::rs2elm::ElmDecode },
        "contribute_decoder",
    );
    let object_pairs_method = pairs_method(&elm_type, &decoder_type, &generics, &type_info);

    let decoder = match type_info {
        TypeInfo::Unit => struct_unit(&elm_type, &decoder_type, &forms),
        TypeInfo::Newtype(field) => struct_newtype(&elm_type, &decoder_type, &forms, &field)?,
        TypeInfo::Tuple(fields) => struct_tuple(&elm_type, &decoder_type, &forms, &fields)?,
        TypeInfo::Struct(fields) => struct_named(
            &elm_type,
            &decoder_type,
            &forms,
            &fields,
            &container_attributes,
        )?,
        TypeInfo::Enum {
            variants,
            #[cfg(feature = "serde")]
            representation,
        } => {
            #[cfg(feature = "serde")]
            let representation = match representation {
                EnumRepresentation::External => enum_external(
                    &elm_type,
                    &decoder_type,
                    &forms,
                    variants,
                    &container_attributes,
                )?,
                EnumRepresentation::Internal { tag } => enum_internal(
                    &elm_type,
                    &decoder_type,
                    &forms,
                    variants,
                    &tag,
                    &container_attributes,
                )?,
                EnumRepresentation::Adjacent { tag, content } => enum_adjacent(
                    &elm_type,
                    &decoder_type,
                    &forms,
                    variants,
                    &tag,
                    &content,
                    &container_attributes,
                )?,
                EnumRepresentation::Untagged => enum_untagged(
                    &elm_type,
                    &decoder_type,
                    &forms,
                    variants,
                    &container_attributes,
                )?,
            };
            #[cfg(not(feature = "serde"))]
            let representation = enum_external(
                &elm_type,
                &decoder_type,
                &forms,
                variants,
                &container_attributes,
            )?;
            representation
        }
    };

    for p in generics.type_params_mut() {
        p.bounds.push(syn::parse_str("::rs2elm::Elm").unwrap());
        p.bounds
            .push(syn::parse_str("::rs2elm::ElmDecode").unwrap());
    }

    let decoder_value = super::polymorph::applied_call(
        &decoder_type,
        &generics,
        quote! { ::rs2elm::ElmDecode },
        "decoder_type",
    );
    let res = quote! {
        impl #generics ::rs2elm::ElmDecode for #ident #generics_without_bounds {
            fn decoder_name() -> ::std::string::String {
                ::std::convert::From::from(#decoder_type)
            }

            fn decoder_type() -> ::std::string::String {
                #decoder_value
            }

            fn decoder_definition() -> ::std::option::Option<::std::string::String> {
                ::std::option::Option::Some(#decoder)
            }

            fn contribute_decoder(module: &mut ::rs2elm::export::Module) {
                <Self as ::rs2elm::Elm>::contribute(module);
                if !module.mark(&<Self as ::rs2elm::ElmDecode>::decoder_name()) {
                    return;
                }
                #contribute_fields
                if let ::std::option::Option::Some(definition) = <Self as ::rs2elm::ElmDecode>::decoder_definition() {
                    module.push_definition(&definition);
                }
            }

            #object_pairs_method
        }
    };
    Ok(res)
}

// =======
// structs
// =======

/// #[derive(Deserialize, Serialize)]
/// struct Unit;
/// "null"
fn struct_unit(elm_type: &str, decoder_type: &str, forms: &Forms) -> TokenStream2 {
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    quote! {::std::format!("\
{decoder_type} : {prefix}
{decoder_type} {binders}=
    Json.Decode.null {elm_type}
",
        elm_type = #elm_type,
        decoder_type = #decoder_type,
        prefix = #prefix,
        binders = #binders,
    )}
}

/// #[derive(Deserialize, Serialize)]
/// struct Newtype(i32);
/// "0"
fn struct_newtype(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    field: &super::PositionalField,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let inner_decoder = elm_decoder(&field.ty, field.lazy, forms)?;
    Ok(quote! {::std::format!("\
{decoder_type} : {prefix}
{decoder_type} {binders}=
    Json.Decode.map {elm_type} ({inner_decoder})
",
        elm_type = #elm_type,
        decoder_type = #decoder_type,
        prefix = #prefix,
        binders = #binders,
        inner_decoder = #inner_decoder,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// struct Tuple(i32, i32);
/// "[0,0]"
fn struct_tuple(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    fields: &[super::PositionalField],
) -> syn::Result<TokenStream2> {
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let decoders = index_steps(fields, "\n        ", forms)?;
    Ok(quote! {::std::format!("\
{decoder_type} : {prefix}
{decoder_type} {binders}=
    Json.Decode.succeed {elm_type}
        {decoders}
",
        elm_type = #elm_type,
        decoder_type = #decoder_type,
        prefix = #prefix,
        binders = #binders,
        decoders = #decoders,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// struct Struct {
///     a: i32,
/// };
/// "{\"a\":0}"
fn struct_named(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let public = if fields.iter().any(|field| field.flatten) {
        flatten_public(elm_type, decoder_type, forms, fields, container_attributes)?
    } else {
        struct_named_direct(elm_type, decoder_type, forms, fields, container_attributes)?
    };
    let pairs = pairs_function(elm_type, decoder_type, forms, fields)?;
    Ok(quote! {
        {
            let mut definition = #public;
            if !definition.ends_with('\n') {
                definition.push('\n');
            }
            definition.push('\n');
            definition.push_str(&#pairs);
            definition
        }
    })
}

fn struct_named_direct(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let mut field_decoders = Vec::new();
    for field in fields {
        field_decoders.push(and_then_field(field, forms)?);
    }
    let recursive = fields.iter().any(|field| field.lazy);
    let names: Vec<String> = fields.iter().map(|field| field.name_elm()).collect();
    let inner = if recursive && !names.is_empty() {
        quote! {::std::format!("\
        Json.Decode.succeed (\\{params} -> {{ {setters} }}) {field_decoders}",
            params = (&[#(#names),*]).join(" "),
            setters = (&[#(::std::format!("{name} = {name}", name = #names)),*]).join(", "),
            field_decoders = (&[#(#field_decoders),*]).join(" "),
        )}
    } else if recursive {
        quote! { ::std::string::String::from("    Json.Decode.succeed {}") }
    } else {
        quote! {::std::format!("\
        Json.Decode.succeed {elm_type} {field_decoders}",
            elm_type = #elm_type,
            field_decoders = (&[#(#field_decoders),*]).join(" "),
        )}
    };
    let expr = with_deny(
        inner,
        &known_keys(fields, &[]),
        deny_unknown(container_attributes),
    );
    let expr = if recursive {
        quote! {
            ::std::format!(
                "Json.Decode.map {elm_type}\n    ({inner})",
                elm_type = #elm_type,
                inner = #expr,
            )
        }
    } else {
        expr
    };
    Ok(quote! {::std::format!("\
{decoder_type} : {prefix}
{decoder_type} {binders}=
    {expr}
",
        decoder_type = #decoder_type,
        prefix = #prefix,
        binders = #binders,
        expr = #expr,
    )})
}

/// Local `take` shared by one decoder. Parent fields read the original list.
const PAIRS_TAKE: &str = r#"    let
        take : String -> List String -> Json.Decode.Decoder a -> Json.Decode.Decoder a -> List (String, Json.Decode.Value) -> Json.Decode.Decoder a
        take primary names missing decoder elmRsRows =
            case List.filter (\( key, _ ) -> List.member key names) elmRsRows of
                [] ->
                    missing
                ( _, found ) :: [] ->
                    case Json.Decode.decodeValue decoder found of
                        Ok got ->
                            Json.Decode.succeed got
                        Err err ->
                            Json.Decode.fail (Json.Decode.errorToString err)
                _ ->
                    Json.Decode.fail ("duplicate field `" ++ primary ++ "`")
    in
"#;

const FINISH_DENY: &str = "case rest of\n                                [] ->\n                                    Json.Decode.succeed value\n                                ( key, _ ) :: _ ->\n                                    Json.Decode.fail (\"unknown field `\" ++ key ++ \"`\")";

fn flatten_public(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = elm_type;
    let chain = object_chain(elm_type, forms, fields, &[], true)?;
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let head = if needs_take(fields) { PAIRS_TAKE } else { "" };
    let finish = if deny_unknown(container_attributes) {
        FINISH_DENY
    } else {
        "Json.Decode.succeed value"
    };
    Ok(quote! {::std::format!("\
{decoder_type} : {prefix}
{decoder_type} {binders}=
{head}    Json.Decode.keyValuePairs Json.Decode.value
        |> Json.Decode.andThen
            (\\elmRsPairs ->
                {chain}
                    |> Json.Decode.andThen
                        (\\( value, rest ) ->
                            {finish}
                        )
            )
",
        decoder_type = #decoder_type,
        prefix = #prefix,
        binders = #binders,
        head = #head,
        chain = #chain,
        finish = #finish,
    )})
}

fn pairs_function(
    elm_type: &str,
    decoder_type: &str,
    forms: &Forms,
    fields: &[StructField],
) -> syn::Result<TokenStream2> {
    let chain = object_chain(elm_type, forms, fields, &[], true)?;
    let pairs_name = format!("{decoder_type}Pairs");
    let binders = &forms.decode_binders;
    let head = if needs_take(fields) { PAIRS_TAKE } else { "" };
    Ok(quote! {::std::format!("\
{pairs_name} {binders}elmRsPairs =
{head}    {chain}
",
        pairs_name = #pairs_name,
        binders = #binders,
        head = #head,
        chain = #chain,
    )})
}

fn pairs_method(
    elm_type: &str,
    decoder_type: &str,
    generics: &syn::Generics,
    type_info: &TypeInfo,
) -> TokenStream2 {
    match type_info {
        TypeInfo::Struct(_) => {
            let pairs_name = format!("{decoder_type}Pairs");
            let applied = super::polymorph::applied_call(
                &pairs_name,
                generics,
                quote! { ::rs2elm::ElmDecode },
                "decoder_type",
            );
            quote! {
                fn object_pairs_decoder() -> ::std::option::Option<::std::string::String> {
                    ::std::option::Option::Some(#applied)
                }
            }
        }
        TypeInfo::Newtype(field) => {
            let ty = &field.ty;
            quote! {
                fn object_pairs_decoder() -> ::std::option::Option<::std::string::String> {
                    <#ty as ::rs2elm::ElmDecode>::object_pairs_decoder().map(|inner| {
                        ::std::format!(
                            "Json.Decode.andThen (\\( elmRsValue, elmRsRest ) -> Json.Decode.succeed ( {elm} elmRsValue, elmRsRest )) << ({inner})",
                            elm = #elm_type,
                            inner = inner,
                        )
                    })
                }
            }
        }
        _ => quote! {},
    }
}

fn needs_take(fields: &[StructField]) -> bool {
    fields
        .iter()
        .any(|field| !field.flatten && !field.codec.skip_deserializing)
}

/// `take` inside an enum decoder's `let`. The binding starts at column 12.
const VARIANT_TAKE: &str = r#"take : String -> List String -> Json.Decode.Decoder a -> Json.Decode.Decoder a -> List (String, Json.Decode.Value) -> Json.Decode.Decoder a
            take primary names missing decoder elmRsRows =
                case List.filter (\( key, _ ) -> List.member key names) elmRsRows of
                    [] ->
                        missing
                    ( _, found ) :: [] ->
                        case Json.Decode.decodeValue decoder found of
                            Ok got ->
                                Json.Decode.succeed got
                            Err err ->
                                Json.Decode.fail (Json.Decode.errorToString err)
                    _ ->
                        Json.Decode.fail ("duplicate field `" ++ primary ++ "`")"#;

const VARIANT_DENY: &str = r#"Maybe.withDefault (Json.Decode.succeed value) (Maybe.map (\( key, _ ) -> Json.Decode.fail ("unknown field `" ++ key ++ "`")) (List.head rest))"#;

fn remember_variant_take(fields: &[StructField], needed: &mut bool) {
    if fields.iter().any(|field| field.flatten) && needs_take(fields) {
        *needed = true;
    }
}

fn push_variant_take(constructors: &mut Vec<TokenStream2>, needed: bool) {
    if needed {
        constructors.push(quote! { ::std::string::String::from(#VARIANT_TAKE) });
    }
}

/// One line, so it can sit inside `oneOf` or a tag case without fighting Elm's indent.
fn variant_object_pipeline(
    variant_name: &str,
    fields: &[StructField],
    extra_claims: &[String],
    deny: bool,
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let ctor = format!("elmRsConstruct{variant_name}");
    let chain = object_chain(&ctor, forms, fields, extra_claims, false)?;
    let finish = if deny {
        VARIANT_DENY
    } else {
        "Json.Decode.succeed value"
    };
    Ok(quote! {
        ::std::format!(
            "Json.Decode.keyValuePairs Json.Decode.value |> Json.Decode.andThen (\\elmRsPairs -> {chain} |> Json.Decode.andThen (\\( value, rest ) -> {finish}))",
            chain = #chain,
            finish = #finish,
        )
    })
}

/// One `andThen` per field. Non-flatten fields read the original list.
/// Each flatten receives the keys still left, and a struct flatten narrows that list.
/// `extra_claims` are keys the parent already consumed, such as an internal enum tag.
/// `record_when_lazy` builds `{Type { field = field }}` for a recursive struct.
/// An enum variant passes false and calls `elmRsConstructVariant` instead.
fn object_chain(
    elm_type: &str,
    forms: &Forms,
    fields: &[StructField],
    extra_claims: &[String],
    record_when_lazy: bool,
) -> syn::Result<TokenStream2> {
    let recursive = record_when_lazy && fields.iter().any(|field| field.lazy);
    let elm_names: Vec<String> = fields.iter().map(|field| field.name_elm()).collect();
    let ctor = if recursive && !elm_names.is_empty() {
        let setters = elm_names
            .iter()
            .map(|name| format!("{name} = {name}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{elm_type} {{ {setters} }}")
    } else if elm_names.is_empty() {
        elm_type.to_string()
    } else {
        format!("{elm_type} {}", elm_names.join(" "))
    };
    let mut claims = claim_keys(fields);
    for claim in extra_claims {
        if !claims.contains(claim) {
            claims.push(claim.clone());
        }
    }
    let mut pool = drop_claims(&claims, "elmRsPairs");
    let mut steps = Vec::new();
    let mut flatten_at = 0usize;
    let mut last_rest = None;
    for field in fields {
        if field.flatten {
            let rest = format!("elmRsRest{flatten_at}");
            flatten_at += 1;
            let ty = &field.ty;
            let pool_now = pool.clone();
            let decoder = quote! {
                ::std::format!(
                    "(({pairs}) ({pool}))",
                    pairs = <#ty as ::rs2elm::ElmDecode>::object_pairs_decoder()
                        .expect("#[serde(flatten)] requires a type whose JSON decoding is an object"),
                    pool = #pool_now,
                )
            };
            let binder = format!("( {}, {rest} )", field.name_elm());
            steps.push((binder, decoder));
            pool = rest.clone();
            last_rest = Some(rest);
            continue;
        }
        if field.codec.skip_deserializing {
            let literal = skip_succeed(field);
            let decoder = quote! { ::std::string::String::from(#literal) };
            steps.push((field.name_elm(), decoder));
            continue;
        }
        let field_decoder = elm_decoder(&field.ty, field.lazy, forms)?;
        let primary = elm_escape(&field.name_decode());
        let names = elm_string_list(&decode_names(field));
        let missing = take_missing(field);
        let decoder = quote! {
            ::std::format!(
                "take \"{primary}\" {names} ({missing}) ({decoder}) elmRsPairs",
                primary = #primary,
                names = #names,
                missing = #missing,
                decoder = #field_decoder,
            )
        };
        steps.push((field.name_elm(), decoder));
    }
    let rest = last_rest.unwrap_or_else(|| drop_claims(&claims, "elmRsPairs"));
    let succeed = quote! {
        ::std::format!("{ctor}, {rest}", ctor = #ctor, rest = #rest)
    };
    Ok(nest_steps(steps, succeed))
}

fn nest_steps(steps: Vec<(String, TokenStream2)>, succeed_body: TokenStream2) -> TokenStream2 {
    let mut acc = quote! {
        ::std::format!("Json.Decode.succeed ( {body} )", body = #succeed_body)
    };
    for (binder, decoder) in steps.into_iter().rev() {
        acc = quote! {
            ::std::format!(
                "Json.Decode.andThen (\\{binder} -> {next}) ({decoder})",
                binder = #binder,
                next = #acc,
                decoder = #decoder,
            )
        };
    }
    acc
}

fn claim_keys(fields: &[StructField]) -> Vec<String> {
    let mut keys = Vec::new();
    for field in fields {
        if field.flatten || field.codec.skip_deserializing {
            continue;
        }
        keys.extend(decode_names(field));
    }
    keys
}

fn drop_claims(claims: &[String], source: &str) -> String {
    if claims.is_empty() {
        return source.to_string();
    }
    let list = elm_string_list(claims);
    format!("List.filter (\\( key, _ ) -> not (List.member key {list})) {source}")
}

fn elm_string_list(names: &[String]) -> String {
    let items = names
        .iter()
        .map(|name| format!("\"{}\"", elm_escape(name)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[ {items} ]")
}

fn elm_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn take_missing(field: &StructField) -> String {
    if let Some(literal) = missing_literal(field) {
        return literal.to_string();
    }
    if type_tokens_are_option(&field.ty) {
        return "Json.Decode.succeed Nothing".to_string();
    }
    format!(
        "Json.Decode.fail \"missing field `{}`\"",
        elm_escape(&field.name_decode())
    )
}

fn skip_succeed(field: &StructField) -> String {
    missing_literal(field)
        .unwrap_or("Json.Decode.fail \"missing default\"")
        .to_string()
}

fn skipped_newtype_literal(field: &super::PositionalField) -> Option<&'static str> {
    if !field.codec.skip_deserializing {
        return None;
    }
    match field.codec.missing {
        Some(super::ElmMissing::Nothing) => Some("Nothing"),
        Some(super::ElmMissing::Unit) => Some("()"),
        None => None,
    }
}

fn skipped_external_arms(variant: &str, names: &[String], missing: &str) -> Vec<TokenStream2> {
    let value = format!("({variant} {missing})");
    names
        .iter()
        .map(|name| {
            let name = name.clone();
            let value = value.clone();
            quote! {
                ::std::format!(
                    "\"{name}\" ->\n                    Json.Decode.succeed {value}",
                    name = #name,
                    value = #value,
                )
            }
        })
        .collect()
}

fn skipped_newtype_external_object(variant: &str, names: &[String], missing: &str) -> TokenStream2 {
    let arms = names
        .iter()
        .map(|name| {
            format!(
                "Json.Decode.map (\\_ -> {variant} {missing}) (Json.Decode.field \"{}\" (Json.Decode.null ()))",
                elm_escape(name),
            )
        })
        .collect::<Vec<_>>();
    let expr = if arms.len() == 1 {
        arms[0].clone()
    } else {
        format!("Json.Decode.oneOf [ {} ]", arms.join(", "))
    };
    quote! { ::std::format!("    {expr}", expr = #expr) }
}

fn skipped_adjacent_body(content: &str, variant: &str, missing: &str) -> String {
    let content = elm_escape(content);
    if missing == "()" {
        format!(
            "Json.Decode.map (\\_ -> {variant} ()) (Json.Decode.field \"{content}\" (Json.Decode.null ()))"
        )
    } else {
        format!(
            "Json.Decode.andThen (\\pairs -> if List.any (\\( name, _ ) -> name == \"{content}\") pairs then Json.Decode.map (\\_ -> {variant} {missing}) (Json.Decode.field \"{content}\" (Json.Decode.null ())) else Json.Decode.succeed ({variant} {missing})) (Json.Decode.keyValuePairs Json.Decode.value)"
        )
    }
}

// =====
// enums
// =====

// everything is contained under the variant field
/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Unit,
///     Newtype(i32),
///     Tuple(i32, i32),
///     Struct { a: i32 },
/// }
/// "\"Unit\""
/// "{\"Newtype\":0}"
/// "{\"Tuple\":[0,0]}"
/// "{\"Struct\":{\"a\":0}}"
fn enum_external(
    elm_type: &str,
    decoder_name: &str,
    forms: &Forms,
    variants: Vec<EnumVariant>,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = elm_type;
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let mut decoders = vec![];
    let mut constructors = vec![];
    let mut needs_variant_take = false;
    let mut unit_arms = vec![];
    let mut other_decoder = None;
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_decode = variant.name_decode();
        #[cfg(feature = "serde")]
        let variant_decode_names = variant.decode_names();
        #[cfg(not(feature = "serde"))]
        let variant_decode_names = vec![elm_name_decode.clone()];
        #[cfg(feature = "serde")]
        if variant.serde_attributes.other {
            other_decoder = Some(quote! {::std::format!("Json.Decode.succeed {}", #elm_name)});
        }

        match &variant.variant {
            EnumVariantKind::Unit => {
                unit_arms.extend(unit_external_arms(&elm_name, &variant_decode_names));
            }
            EnumVariantKind::Newtype(inner) => {
                if let Some(missing) = skipped_newtype_literal(inner) {
                    unit_arms.extend(skipped_external_arms(
                        &elm_name,
                        &variant_decode_names,
                        missing,
                    ));
                    decoders.push(skipped_newtype_external_object(
                        &elm_name,
                        &variant_decode_names,
                        missing,
                    ));
                } else {
                    decoders.push(enum_variant_newtype_external(
                        &elm_name,
                        &variant_decode_names,
                        &inner.ty,
                        inner.lazy,
                        forms,
                    )?);
                }
            }
            EnumVariantKind::Tuple(types) => {
                decoders.push(enum_variant_tuple_external(
                    &elm_name,
                    &variant_decode_names,
                    types,
                    forms,
                )?);
            }
            EnumVariantKind::Struct(fields) => {
                remember_variant_take(fields, &mut needs_variant_take);
                let (decoder, constructor) = enum_variant_struct_external(
                    &elm_name,
                    &variant_decode_names,
                    fields,
                    container_attributes,
                    forms,
                )?;
                constructors.push(constructor);
                decoders.push(decoder);
            }
        }
    }
    // Every unit variant shares one string case. A pure unit enum is that case.
    // A mixed enum keeps it as the first `oneOf` branch.
    let unit_only = !unit_arms.is_empty()
        && decoders.is_empty()
        && constructors.is_empty()
        && other_decoder.is_none();
    if !unit_arms.is_empty() && !unit_only {
        decoders.insert(0, unit_external_case(&unit_arms));
    }
    if let Some(other_decoder) = other_decoder {
        decoders.push(other_decoder);
    }

    push_variant_take(&mut constructors, needs_variant_take);
    let constructors = if constructors.is_empty() {
        quote! {""}
    } else {
        quote! {::std::format!("
        let
            {}
        in", (&[
                    #(#constructors),*
            ]).join("\n            ")
        )}
    };

    if unit_only {
        let case = unit_external_case(&unit_arms);
        return Ok(quote! {::std::format!("\
{decoder_name} : {prefix}
{decoder_name} {binders}=
    {case}",
            decoder_name = #decoder_name,
            prefix = #prefix,
            binders = #binders,
            case = #case,
        )});
    }

    Ok(quote! {::std::format!("\
{decoder_name} : {prefix}
{decoder_name} {binders}= {constructors}
    Json.Decode.oneOf
        [ {decoders}
        ]",
        decoder_name = #decoder_name,
        prefix = #prefix,
        binders = #binders,
        constructors = #constructors,
        decoders = (
            &[
                #(#decoders),*
            ]
        ).join("\n        , ")
    )})
}

/// One case arm per accepted tag. Aliases share the body. The wildcard, if any, stays last.
#[cfg(feature = "serde")]
fn tag_arms(names: &[String], body: TokenStream2) -> TokenStream2 {
    let labels = names.to_vec();
    if names.len() == 1 {
        let name = labels[0].clone();
        return quote! {
            ::std::format!(
                "\"{name}\" ->\n                        {body}",
                name = #name,
                body = #body,
            )
        };
    }
    quote! {
        {
            let body = #body;
            (&[
                #(
                    ::std::format!(
                        "\"{name}\" ->\n                        {body}",
                        name = #labels,
                        body = body,
                    )
                ),*
            ]).join("\n                    ")
        }
    }
}

/// `#[serde(other)]` is a wildcard arm. It has to be last, or elm repl reports an unfinished case.
#[cfg(feature = "serde")]
fn tag_fallback(other: Option<TokenStream2>) -> TokenStream2 {
    other.unwrap_or_else(|| {
        quote! {
            ::std::string::String::from(
                "unexpected ->\n                        Json.Decode.fail <| \"Unexpected variant \" ++ unexpected",
            )
        }
    })
}

// an object with a tag field
/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t")]
/// enum Internal {
///     Unit,
///     Struct { a: i32 },
/// }
/// "{\"t\":\"Unit\"}"
/// "{\"t\":\"Struct\",\"a\":0}"
#[cfg(feature = "serde")]
fn enum_internal(
    elm_type: &str,
    decoder_name: &str,
    forms: &Forms,
    variants: Vec<EnumVariant>,
    tag: &str,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = elm_type;
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let mut decoders = vec![];
    let mut constructors = vec![];
    let mut needs_variant_take = false;
    let mut other_decoder = None;
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_decode = variant.name_decode();
        let variant_decode_names = variant.decode_names();
        #[cfg(feature = "serde")]
        if variant.serde_attributes.other {
            other_decoder = Some(quote! {::std::format!("\
_ ->
                        Json.Decode.succeed {}", #elm_name)});
        }

        let decoder = match &variant.variant {
            EnumVariantKind::Unit => {
                enum_variant_unit_internal_or_adjacent(&elm_name, &variant_decode_names)
            }
            EnumVariantKind::Newtype(_) => {
                return Err(syn::Error::new(
                    variant.span,
                    "Internally tagged newtype variants are not supported by serde_json",
                ));
            }
            EnumVariantKind::Tuple(_) => {
                return Err(syn::Error::new(
                    variant.span,
                    "Internally tagged tuple variants are not supported by serde_json",
                ));
            }
            EnumVariantKind::Struct(fields) => {
                remember_variant_take(fields, &mut needs_variant_take);
                let (decoder, constructor) = enum_variant_struct_internal(
                    &tag,
                    &elm_name,
                    &variant_decode_names,
                    fields,
                    container_attributes,
                    forms,
                )?;
                constructors.push(constructor);
                decoder
            }
        };
        decoders.push(decoder);
    }
    let fallback = tag_fallback(other_decoder);

    push_variant_take(&mut constructors, needs_variant_take);
    let constructors = if constructors.is_empty() {
        quote! {""}
    } else {
        quote! {::std::format!("
        let
            {}
        in", (&[
                    #(#constructors),*
            ]).join("\n            ")
        )}
    };

    let decoder = quote! {::std::format!("\
{decoder_name} : {prefix}
{decoder_name} {binders}= {constructors}
    Json.Decode.field \"{tag}\" Json.Decode.string
        |> Json.Decode.andThen
            (\\elmRsTag ->
                case elmRsTag of
                    {decoders}
                    {fallback}
            )",
        decoder_name = #decoder_name,
        prefix = #prefix,
        binders = #binders,
        constructors = #constructors,
        tag = #tag,
        decoders = (
            &[
                #(#decoders),*
            ]
        ).join("\n                    "),
        fallback = #fallback,
    )};

    Ok(decoder)
}

// an object with tag and content fields
/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Unit,
///     Newtype(i32),
///     Tuple(i32, i32),
///     Struct { a: i32 },
/// }
/// "{\"t\":\"Unit\"}"
/// "{\"t\":\"Newtype\",\"c\":0}"
/// "{\"t\":\"Tuple\",\"c\":[0,0]}"
/// "{\"t\":\"Struct\",\"c\":{\"a\":0}}"
#[cfg(feature = "serde")]
fn enum_adjacent(
    elm_type: &str,
    decoder_name: &str,
    forms: &Forms,
    variants: Vec<EnumVariant>,
    tag: &str,
    content: &str,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = elm_type;
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let mut decoders = vec![];
    let mut constructors = vec![];
    let mut needs_variant_take = false;
    let mut other_decoder = None;
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_decode = variant.name_decode();
        let variant_decode_names = variant.decode_names();
        #[cfg(feature = "serde")]
        if variant.serde_attributes.other {
            other_decoder = Some(quote! {::std::format!("\
_ ->
                        Json.Decode.succeed {}", #elm_name)});
        }

        let decoder = match &variant.variant {
            EnumVariantKind::Unit => {
                enum_variant_unit_internal_or_adjacent(&elm_name, &variant_decode_names)
            }
            EnumVariantKind::Newtype(inner) => enum_variant_newtype_adjacent(
                content,
                &elm_name,
                &variant_decode_names,
                inner,
                forms,
            )?,
            EnumVariantKind::Tuple(types) => enum_variant_tuple_adjacent(
                content,
                &elm_name,
                &variant_decode_names,
                types,
                forms,
            )?,
            EnumVariantKind::Struct(fields) => {
                remember_variant_take(fields, &mut needs_variant_take);
                let (decoder, constructor) = enum_variant_struct_adjacent(
                    content,
                    &elm_name,
                    &variant_decode_names,
                    fields,
                    container_attributes,
                    forms,
                )?;
                constructors.push(constructor);
                decoder
            }
        };
        decoders.push(decoder);
    }
    let fallback = tag_fallback(other_decoder);

    push_variant_take(&mut constructors, needs_variant_take);
    let constructors = if constructors.is_empty() {
        quote! {""}
    } else {
        quote! {::std::format!("
        let
            {}
        in", (&[
                    #(#constructors),*
            ]).join("\n            ")
        )}
    };

    let decoder = quote! {::std::format!("\
{decoder_name} : {prefix}
{decoder_name} {binders}= {constructors}
    Json.Decode.field \"{tag}\" Json.Decode.string
        |> Json.Decode.andThen
            (\\elmRsTag ->
                case elmRsTag of
                    {decoders}
                    {fallback}
            )",
        decoder_name = #decoder_name,
        prefix = #prefix,
        binders = #binders,
        constructors = #constructors,
        tag = #tag,
        decoders = (
            &[
                #(#decoders),*
            ]
        ).join("\n                    "),
        fallback = #fallback,
    )};
    Ok(decoder)
}

// no tag
/// #[derive(Deserialize, Serialize)]
/// #[serde(untagged)]
/// enum Untagged {
///     Unit,
///     Newtype(i32),
///     Tuple(i32, i32),
///     Struct { a: i32 },
/// }
/// "null"
/// "0"
/// "[0,0]"
/// "{\"a\":0}"
#[cfg(feature = "serde")]
fn enum_untagged(
    elm_type: &str,
    decoder_name: &str,
    forms: &Forms,
    variants: Vec<EnumVariant>,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = elm_type;
    let prefix = &forms.decode_prefix;
    let binders = &forms.decode_binders;
    let mut decoders = vec![];
    let mut constructors = vec![];
    let mut needs_variant_take = false;
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let decoder = match &variant.variant {
            EnumVariantKind::Unit => enum_variant_unit_untagged(&elm_name),
            EnumVariantKind::Newtype(inner) => {
                enum_variant_newtype_untagged(&elm_name, inner, forms)?
            }
            EnumVariantKind::Tuple(types) => enum_variant_tuple_untagged(&elm_name, types, forms)?,
            EnumVariantKind::Struct(fields) => {
                remember_variant_take(fields, &mut needs_variant_take);
                let (decoder, constructor) =
                    enum_variant_struct_untagged(&elm_name, fields, container_attributes, forms)?;
                constructors.push(constructor);
                decoder
            }
        };
        decoders.push(decoder);
    }

    push_variant_take(&mut constructors, needs_variant_take);
    let constructors = if constructors.is_empty() {
        quote! {""}
    } else {
        quote! {::std::format!("
        let
            {}
        in", (&[
                    #(#constructors),*
            ]).join("\n            ")
        )}
    };

    let decoder = quote! {::std::format!("\
{decoder_name} : {prefix}
{decoder_name} {binders}= {constructors}
    Json.Decode.oneOf
        [ {decoders}
        ]",
        decoder_name = #decoder_name,
        prefix = #prefix,
        binders = #binders,
        constructors = #constructors,
        decoders = (
            &[
                #(#decoders),*
            ]
        ).join("\n    , ")
    )};

    Ok(decoder)
}

// =================
// external variants
// =================

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Unit,
/// }
/// "\"Unit\""
fn unit_external_arms(variant_name: &str, names: &[String]) -> Vec<TokenStream2> {
    names
        .iter()
        .map(|name| {
            quote! {
                ::std::format!(
                    "\"{name}\" ->\n                    Json.Decode.succeed {variant}",
                    name = #name,
                    variant = #variant_name,
                )
            }
        })
        .collect()
}

fn unit_external_case(arms: &[TokenStream2]) -> TokenStream2 {
    quote! {::std::format!("\
Json.Decode.string
    |> Json.Decode.andThen
        (\\x ->
            case x of
                {arms}
                unexpected ->
                    Json.Decode.fail <| \"Unexpected variant \" ++ unexpected
        )",
        arms = (&[#(#arms),*]).join("\n                "),
    )}
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Newtype(i32),
/// }
/// One JSON object key, or `oneOf` when a variant `alias` accepts more than the serialize name.
fn keyed_content(names: &[String], inner: TokenStream2) -> TokenStream2 {
    if names.len() == 1 {
        let name = &names[0];
        return quote! {
            ::std::format!(
                "Json.Decode.field \"{name}\" ({inner})",
                name = #name,
                inner = #inner,
            )
        };
    }
    let name_literals = names.to_vec();
    quote! {
        {
            let inner = #inner;
            ::std::format!(
                "Json.Decode.oneOf [ {} ]",
                (&[
                    #(
                        ::std::format!(
                            "Json.Decode.field \"{name}\" ({inner})",
                            name = #name_literals,
                            inner = inner,
                        )
                    ),*
                ]).join(", ")
            )
        }
    }
}

/// "{\"Newtype\":0}"
fn enum_variant_newtype_external(
    variant_name: &str,
    names: &[String],
    inner_type: &TokenStream2,
    lazy: bool,
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let decoder = elm_decoder(inner_type, lazy, forms)?;
    let keyed = keyed_content(names, decoder);
    Ok(quote! {::std::format!("\
    Json.Decode.map {enum_variant} ({keyed})",
        enum_variant = #variant_name,
        keyed = #keyed,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Tuple(i32, i32),
/// }
/// "{\"Tuple\":[0,0]}"
fn enum_variant_tuple_external(
    variant_name: &str,
    names: &[String],
    fields: &[super::PositionalField],
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let decoders = index_steps(fields, " ", forms)?;
    let inner = quote! {::std::format!("\
    Json.Decode.succeed {variant_name} {decoders}",
        variant_name = #variant_name,
        decoders = #decoders,
    )};
    let keyed = keyed_content(names, inner);
    Ok(quote! {::std::format!("    {keyed}", keyed = #keyed)})
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Struct { a: i32 },
/// }
/// "{\"Struct\":{\"a\":0}}"
fn enum_variant_struct_external(
    variant_name: &str,
    names: &[String],
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let constructor = constructor(variant_name, &field_names);
    let inner = if fields.iter().any(|field| field.flatten) {
        variant_object_pipeline(
            variant_name,
            fields,
            &[],
            deny_unknown(container_attributes),
            forms,
        )?
    } else {
        let mut field_decoders = Vec::new();
        for field in fields {
            field_decoders.push(and_then_field(field, forms)?);
        }
        let inner = quote! {::std::format!("\
        Json.Decode.succeed elmRsConstruct{variant_name} {decoders}",
            variant_name = #variant_name,
            decoders = (&[#(#field_decoders),*]).join(" "),
        )};
        with_deny(
            inner,
            &known_keys(fields, &[]),
            deny_unknown(container_attributes),
        )
    };

    let keyed = keyed_content(names, inner);
    let decoder = quote! {::std::format!("    {keyed}", keyed = #keyed)};

    Ok((decoder, constructor))
}

// =================
// internal variants
// =================

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t")]
/// enum Internal {
///     Unit,
/// }
/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Unit,
/// }
/// "{\"t\":\"Unit\"}"
#[cfg(feature = "serde")]
fn enum_variant_unit_internal_or_adjacent(variant_name: &str, names: &[String]) -> TokenStream2 {
    let arms: Vec<TokenStream2> = names
        .iter()
        .map(|name| {
            quote! {
                ::std::format!(
                    "\"{name}\" ->\n                        Json.Decode.succeed {variant}",
                    name = #name,
                    variant = #variant_name,
                )
            }
        })
        .collect();
    quote! { (&[#(#arms),*]).join("\n                    ") }
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t")]
/// enum Internal {
///     Struct { a: i32 },
/// }
/// "{\"t\":\"Struct\",\"a\":0}"
#[cfg(feature = "serde")]
fn enum_variant_struct_internal(
    tag: &str,
    variant_name: &str,
    names: &[String],
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let constructor = constructor(variant_name, &field_names);
    let inner = if fields.iter().any(|field| field.flatten) {
        let extra = vec![tag.to_string()];
        variant_object_pipeline(
            variant_name,
            fields,
            &extra,
            deny_unknown(container_attributes),
            forms,
        )?
    } else {
        let mut field_decoders = Vec::new();
        for field in fields {
            field_decoders.push(and_then_field(field, forms)?);
        }
        let inner = quote! {::std::format!("\
        Json.Decode.succeed elmRsConstruct{variant_name} {decoders}",
            variant_name = #variant_name,
            decoders = (&[#(#field_decoders),*]).join(" "),
        )};
        with_deny(
            inner,
            &known_keys(fields, &[tag]),
            deny_unknown(container_attributes),
        )
    };

    Ok((tag_arms(names, inner), constructor))
}

// #################
// adjacent variants
// #################

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Newtype(i32),
/// }
/// "{\"t\":\"Newtype\",\"c\":0}"
#[cfg(feature = "serde")]
fn enum_variant_newtype_adjacent(
    content: &str,
    variant_name: &str,
    names: &[String],
    field: &super::PositionalField,
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    if let Some(missing) = skipped_newtype_literal(field) {
        let body = skipped_adjacent_body(content, variant_name, missing);
        return Ok(tag_arms(
            names,
            quote! { ::std::string::String::from(#body) },
        ));
    }
    let decoder = elm_decoder(&field.ty, field.lazy, forms)?;
    let body = quote! {::std::format!("\
    Json.Decode.map {variant_name} (Json.Decode.field \"{content}\" ({decoder}))",
        variant_name = #variant_name,
        content = #content,
        decoder = #decoder,
    )};
    Ok(tag_arms(names, body))
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Tuple(i32, i32),
/// }
/// "{\"t\":\"Tuple\",\"c\":[0,0]}"
#[cfg(feature = "serde")]
fn enum_variant_tuple_adjacent(
    content: &str,
    variant_name: &str,
    names: &[String],
    fields: &[super::PositionalField],
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let decoders = index_steps(fields, " ", forms)?;
    let body = quote! {::std::format!("\
    Json.Decode.field \"{content}\" (Json.Decode.succeed {variant_name} {decoders})",
        variant_name = #variant_name,
        content = #content,
        decoders = #decoders,
    )};
    Ok(tag_arms(names, body))
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Struct { a: i32 },
/// }
/// "{\"t\":\"Struct\",\"c\":{\"a\":0}}"
#[cfg(feature = "serde")]
fn enum_variant_struct_adjacent(
    content: &str,
    variant_name: &str,
    names: &[String],
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    let field_names = fields
        .iter()
        .map(|field| field.name_elm())
        .collect::<Vec<_>>();
    let constructor = constructor(variant_name, &field_names);

    let inner = if fields.iter().any(|field| field.flatten) {
        variant_object_pipeline(
            variant_name,
            fields,
            &[],
            deny_unknown(container_attributes),
            forms,
        )?
    } else {
        let mut field_decoders = Vec::new();
        for field in fields {
            field_decoders.push(and_then_field(field, forms)?);
        }
        let inner = quote! {::std::format!("\
        Json.Decode.succeed elmRsConstruct{variant_name} {field_decoders}",
        variant_name = #variant_name,
        field_decoders = (&[#(#field_decoders),*]).join(" "),
        )};
        with_deny(
            inner,
            &known_keys(fields, &[]),
            deny_unknown(container_attributes),
        )
    };
    let body = quote! {::std::format!("\
    Json.Decode.field \"{content}\" ({inner})",
        content = #content,
        inner = #inner,
    )};

    Ok((tag_arms(names, body), constructor))
}

// #################
// untagged variants
// #################

/// #[derive(Deserialize, Serialize)]
/// #[serde(untagged)]
/// enum Untagged {
///     Unit,
/// }
/// "null"
#[cfg(feature = "serde")]
fn enum_variant_unit_untagged(variant_name: &str) -> TokenStream2 {
    quote! {::std::format!("\
    Json.Decode.null {variant_name}",
        variant_name = #variant_name
    )}
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(untagged)]
/// enum Untagged {
///     Newtype(i32),
/// }
/// "0"
#[cfg(feature = "serde")]
fn enum_variant_newtype_untagged(
    variant_name: &str,
    field: &super::PositionalField,
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    if let Some(missing) = skipped_newtype_literal(field) {
        let body =
            format!("    Json.Decode.map (\\_ -> {variant_name} {missing}) (Json.Decode.null ())");
        return Ok(quote! { ::std::string::String::from(#body) });
    }
    let decoder = elm_decoder(&field.ty, field.lazy, forms)?;
    Ok(quote! {::std::format!("\
    Json.Decode.map {variant_name} ({decoder})",
        variant_name = #variant_name,
        decoder = #decoder,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(untagged)]
/// enum Untagged {
///     Tuple(i32, i32),
/// }
/// "[0,0]"
#[cfg(feature = "serde")]
fn enum_variant_tuple_untagged(
    variant_name: &str,
    fields: &[super::PositionalField],
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let decoders = index_steps(fields, " ", forms)?;
    Ok(quote! {::std::format!("\
    Json.Decode.succeed {enum_variant} {decoders}",
        enum_variant = #variant_name,
        decoders = #decoders,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(untagged)]
/// enum Untagged {
///     Struct { a: i32 },
/// }
/// "{\"a\":0}"
#[cfg(feature = "serde")]
fn enum_variant_struct_untagged(
    variant_name: &str,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
) -> syn::Result<(TokenStream2, TokenStream2)> {
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let constructor = constructor(variant_name, &field_names);
    let decoder = if fields.iter().any(|field| field.flatten) {
        variant_object_pipeline(
            variant_name,
            fields,
            &[],
            deny_unknown(container_attributes),
            forms,
        )?
    } else {
        let mut field_decoders = Vec::new();
        for field in fields {
            field_decoders.push(and_then_field(field, forms)?);
        }
        let inner = quote! {::std::format!("\
        Json.Decode.succeed elmRsConstruct{variant_name} {decoders}",
            variant_name = #variant_name,
            decoders = (&[#(#field_decoders),*]).join(" "),
        )};
        with_deny(
            inner,
            &known_keys(fields, &[]),
            deny_unknown(container_attributes),
        )
    };

    Ok((decoder, constructor))
}

// #######
// helpers
// #######

fn decode_names(field: &StructField) -> Vec<String> {
    #[cfg(feature = "serde")]
    {
        field.decode_names()
    }
    #[cfg(not(feature = "serde"))]
    {
        vec![field.name_decode()]
    }
}

fn elm_decoder(ty: &TokenStream2, lazy: bool, forms: &Forms) -> syn::Result<TokenStream2> {
    let inner = super::polymorph::decoder_expr(ty, &forms.params)?;
    if lazy {
        Ok(quote! {
            ::std::format!("Json.Decode.lazy (\\_ -> ({}))", #inner)
        })
    } else {
        Ok(inner)
    }
}

fn index_steps(
    fields: &[super::PositionalField],
    separator: &str,
    forms: &Forms,
) -> syn::Result<TokenStream2> {
    let mut steps = Vec::new();
    let mut seq = 0usize;
    let mut minimum = 0usize;
    let mut maximum = 0usize;
    for field in fields {
        if field.codec.skip_deserializing {
            let literal =
                positional_missing(field).unwrap_or("Json.Decode.fail \"missing default\"");
            let step = format!("|> Json.Decode.andThen (\\x -> {literal} |> Json.Decode.map x)");
            steps.push(quote! { ::std::string::String::from(#step) });
            continue;
        }
        let decoder = elm_decoder(&field.ty, field.lazy, forms)?;
        if let Some(literal) = positional_missing(field) {
            maximum += 1;
            let index = seq;
            steps.push(quote! {
                ::std::format!(
                    "|> Json.Decode.andThen (\\x -> Json.Decode.andThen (\\items -> if List.length items > {index} then Json.Decode.index {index} ({decoder}) else {literal}) (Json.Decode.list Json.Decode.value) |> Json.Decode.map x)",
                    index = #index,
                    decoder = #decoder,
                    literal = #literal,
                )
            });
        } else {
            minimum += 1;
            maximum += 1;
            let index = seq;
            steps.push(quote! {
                ::std::format!(
                    "|> Json.Decode.andThen (\\x -> Json.Decode.index {idx} ({decoder}) |> Json.Decode.map x)",
                    idx = #index,
                    decoder = #decoder,
                )
            });
        }
        seq += 1;
    }
    let guard = if minimum == maximum {
        format!(
            "|> Json.Decode.andThen (\\kept -> Json.Decode.andThen (\\items -> if List.length items == {maximum} then Json.Decode.succeed kept else Json.Decode.fail \"expected a list of length {maximum}\") (Json.Decode.list Json.Decode.value))"
        )
    } else {
        format!(
            "|> Json.Decode.andThen (\\kept -> Json.Decode.andThen (\\items -> if List.length items >= {minimum} && List.length items <= {maximum} then Json.Decode.succeed kept else Json.Decode.fail \"expected a list of length {minimum} to {maximum}\") (Json.Decode.list Json.Decode.value))"
        )
    };
    steps.insert(0, quote! { ::std::string::String::from(#guard) });
    Ok(quote! { (&[#(#steps),*]).join(#separator) })
}

fn positional_missing(field: &super::PositionalField) -> Option<&'static str> {
    #[cfg(feature = "serde")]
    {
        match field.codec.missing {
            Some(super::ElmMissing::Nothing) => Some("Json.Decode.succeed Nothing"),
            Some(super::ElmMissing::Unit) => Some("Json.Decode.succeed ()"),
            None => None,
        }
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = field;
        None
    }
}

fn field_decoder_expr(field: &StructField, forms: &Forms) -> syn::Result<TokenStream2> {
    if field.flatten {
        return elm_decoder(&field.ty, field.lazy, forms);
    }
    if field.codec.skip_deserializing {
        let literal = missing_literal(field).unwrap_or("Json.Decode.fail \"missing default\"");
        return Ok(quote! { ::std::string::String::from(#literal) });
    }
    let ty = &field.ty;
    let names = decode_names(field);
    let decoder = elm_decoder(ty, field.lazy, forms)?;
    let read = read_named_field(&names, decoder);
    // serde treats a missing `Option` key as `None`, and `default` the same way.
    // A present value of the wrong type still fails. `oneOf` with `succeed`
    // would hide that failure.
    if let Some(literal) = missing_literal(field).or_else(|| {
        if type_tokens_are_option(ty) {
            Some("Json.Decode.succeed Nothing")
        } else {
            None
        }
    }) {
        return Ok(field_when_present(&names, read, literal));
    }
    Ok(read)
}

fn read_named_field(names: &[String], decoder: TokenStream2) -> TokenStream2 {
    let labels = names.to_vec();
    if names.len() == 1 {
        let name = labels[0].clone();
        return quote! {
            ::std::format!(
                "Json.Decode.field \"{name}\" ({decoder})",
                name = #name,
                decoder = #decoder,
            )
        };
    }
    quote! {
        {
            let decoder = #decoder;
            ::std::format!(
                "Json.Decode.oneOf [ {} ]",
                (&[
                    #(
                        ::std::format!(
                            "Json.Decode.field \"{name}\" ({decoder})",
                            name = #labels,
                            decoder = decoder,
                        )
                    ),*
                ]).join(", ")
            )
        }
    }
}

fn field_when_present(names: &[String], read: TokenStream2, fallback: &str) -> TokenStream2 {
    let checks = names
        .iter()
        .map(|name| {
            format!(
                "name == \"{}\"",
                name.replace('\\', "\\\\").replace('"', "\\\"")
            )
        })
        .collect::<Vec<_>>()
        .join(" || ");
    quote! {
        {
            let read = #read;
            ::std::format!(
                "(Json.Decode.andThen (\\pairs -> if List.any (\\( name, _ ) -> {checks}) pairs then {read} else {fallback}) (Json.Decode.keyValuePairs Json.Decode.value))",
                checks = #checks,
                read = read,
                fallback = #fallback,
            )
        }
    }
}

fn type_tokens_are_option(ty: &TokenStream2) -> bool {
    syn::parse2::<syn::Type>(ty.clone())
        .ok()
        .is_some_and(|ty| super::type_is_named(&ty, "Option"))
}

fn missing_literal(field: &StructField) -> Option<&'static str> {
    #[cfg(feature = "serde")]
    {
        return match field.codec.missing {
            Some(super::ElmMissing::Nothing) => Some("Json.Decode.succeed Nothing"),
            Some(super::ElmMissing::Unit) => Some("Json.Decode.succeed ()"),
            None => None,
        };
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = field;
        None
    }
}

fn deny_unknown(container: &ContainerAttributes) -> bool {
    #[cfg(feature = "serde")]
    {
        container.serde.deny_unknown_fields
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = container;
        false
    }
}

fn known_keys(fields: &[StructField], extra: &[&str]) -> String {
    let mut keys: Vec<String> = extra.iter().map(|key| (*key).to_string()).collect();
    for field in fields {
        if field.codec.skip_deserializing {
            continue;
        }
        keys.extend(decode_names(field));
    }
    keys.iter()
        .map(|key| format!("\"{}\"", key.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn with_deny(inner: TokenStream2, keys: &str, deny: bool) -> TokenStream2 {
    if !deny {
        return inner;
    }
    quote! {
        ::std::format!(
            "Json.Decode.map2 (\\_ value -> value)\n    (Json.Decode.andThen\n        (\\pairs ->\n            case List.filter (\\( key, _ ) -> not (List.member key [ {keys} ])) pairs of\n                [] ->\n                    Json.Decode.succeed ()\n                ( key, _ ) :: _ ->\n                    Json.Decode.fail (\"unknown field `\" ++ key ++ \"`\")\n        )\n        (Json.Decode.keyValuePairs Json.Decode.value)\n    )\n    ({inner})",
            keys = #keys,
            inner = #inner,
        )
    }
}

fn and_then_field(field: &StructField, forms: &Forms) -> syn::Result<TokenStream2> {
    let decoder = field_decoder_expr(field, forms)?;
    Ok(
        quote! { ::std::format!("|> Json.Decode.andThen (\\x -> Json.Decode.map x ({}))", #decoder) },
    )
}

fn constructor(variant_name: &str, field_names: &[String]) -> TokenStream2 {
    quote! {::std::format!("\
elmRsConstruct{enum_variant} {fields} =
                        {enum_variant} {{ {setters} }}",
        enum_variant = #variant_name,
        fields = (
            &[
                #(::std::format!("{}",
                    #field_names
                )),*
            ]
        ).join(" "),
        setters = (
            &[
                #(::std::format!("{0} = {0}",
                    #field_names
                )),*
            ]
        ).join(", "),
    )}
}
