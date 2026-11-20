#![allow(unused_variables)]
//! Derive macro for ElmEncode.

use super::{
    EnumVariantKind, Intermediate, TypeInfo,
    polymorph::{self, Ctx, Forms},
};
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
    let encoder_type = format!("{}Encoder", elm_type.to_lower_camel_case());
    let forms = Forms::from_generics(&elm_type, &generics);
    let contribute_fields = super::contribute_calls(
        &super::dependency_types(&type_info),
        quote! { ::rs2elm::ElmEncode },
        "contribute_encoder",
    );

    let (encoder, object_fields_name, newtype_fields) = match type_info {
        TypeInfo::Unit => (struct_unit(&encoder_type, &forms), None, None),
        TypeInfo::Newtype(field) => (
            struct_newtype(&elm_type, &encoder_type, &forms, &ident, &field.ty)?,
            None,
            Some(field.ty),
        ),
        TypeInfo::Tuple(fields) => (
            struct_tuple(&elm_type, &encoder_type, &forms, &ident, &fields)?,
            None,
            None,
        ),
        TypeInfo::Struct(fields) => {
            let fields_name = format!("{encoder_type}Fields");
            let recursive = fields.iter().any(|field| field.lazy);
            (
                struct_named(
                    &elm_type,
                    &encoder_type,
                    &fields_name,
                    &forms,
                    &ident,
                    &fields,
                    recursive,
                    &container_attributes,
                )?,
                Some(fields_name),
                None,
            )
        }
        TypeInfo::Enum {
            variants,
            #[cfg(feature = "serde")]
            representation,
        } => {
            #[cfg(feature = "serde")]
            let generated = match representation {
                EnumRepresentation::External => (
                    enum_external(
                        &encoder_type,
                        &forms,
                        &ident,
                        variants,
                        &container_attributes,
                    )?,
                    None,
                    None,
                ),
                EnumRepresentation::Internal { tag } => {
                    let fields_name = format!("{encoder_type}Fields");
                    (
                        enum_internal(
                            &encoder_type,
                            &forms,
                            &ident,
                            variants,
                            &tag,
                            &fields_name,
                            &container_attributes,
                        )?,
                        Some(fields_name),
                        None,
                    )
                }
                EnumRepresentation::Adjacent { tag, content } => (
                    enum_adjacent(
                        &encoder_type,
                        &forms,
                        &ident,
                        variants,
                        &tag,
                        &content,
                        &container_attributes,
                    )?,
                    None,
                    None,
                ),
                EnumRepresentation::Untagged => (
                    enum_untagged(
                        &encoder_type,
                        &forms,
                        &ident,
                        variants,
                        &container_attributes,
                    )?,
                    None,
                    None,
                ),
            };
            #[cfg(not(feature = "serde"))]
            let generated = (
                enum_external(
                    &encoder_type,
                    &forms,
                    &ident,
                    variants,
                    &container_attributes,
                )?,
                None,
                None,
            );
            generated
        }
    };

    for p in generics.type_params_mut() {
        p.bounds.push(syn::parse_str("::rs2elm::Elm").unwrap());
        p.bounds
            .push(syn::parse_str("::rs2elm::ElmEncode").unwrap());
    }

    let encoder_value = polymorph::applied_call(
        &encoder_type,
        &generics,
        quote! { ::rs2elm::ElmEncode },
        "encoder_type",
    );
    let object_fields_method = match &object_fields_name {
        Some(fields_name) => {
            let applied = polymorph::applied_call(
                fields_name,
                &generics,
                quote! { ::rs2elm::ElmEncode },
                "encoder_type",
            );
            quote! {
                fn object_fields_name() -> ::std::option::Option<::std::string::String> {
                    ::std::option::Option::Some(::std::convert::From::from(#fields_name))
                }

                fn object_fields_encoder() -> ::std::option::Option<::std::string::String> {
                    ::std::option::Option::Some(#applied)
                }
            }
        }
        None => quote! {},
    };
    let newtype_fields_method = match newtype_fields {
        Some(ty) => quote! {
            fn object_fields_encoder() -> ::std::option::Option<::std::string::String> {
                <#ty as ::rs2elm::ElmEncode>::object_fields_encoder().map(|encoder| {
                    ::std::format!(
                        "(\\({elm} elmRsInner) -> ({encoder}) elmRsInner)",
                        elm = #elm_type,
                        encoder = encoder,
                    )
                })
            }
        },
        None => quote! {},
    };
    let res = quote! {
        impl #generics ::rs2elm::ElmEncode for #ident #generics_without_bounds {
            fn encoder_name() -> ::std::string::String {
                ::std::convert::From::from(#encoder_type)
            }

            fn encoder_type() -> ::std::string::String {
                #encoder_value
            }

            fn encoder_definition() -> ::std::option::Option<::std::string::String> {
                ::std::option::Option::Some(#encoder)
            }

            fn contribute_encoder(module: &mut ::rs2elm::export::Module) {
                <Self as ::rs2elm::Elm>::contribute(module);
                if !module.mark(&<Self as ::rs2elm::ElmEncode>::encoder_name()) {
                    return;
                }
                #contribute_fields
                if let ::std::option::Option::Some(definition) = <Self as ::rs2elm::ElmEncode>::encoder_definition() {
                    module.push_definition(&definition);
                }
            }

            #object_fields_method
            #newtype_fields_method
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
fn struct_unit(encoder_type: &str, forms: &Forms) -> TokenStream2 {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    quote! {::std::format!("\
{encoder_type} : {prefix} -> Json.Encode.Value
{encoder_type} {binders}_ =
    Json.Encode.null
",
        prefix = #prefix,
        binders = #binders,
        encoder_type = #encoder_type,
    )}
}

/// #[derive(Deserialize, Serialize)]
/// struct Newtype(i32);
/// "0"
fn struct_newtype(
    elm_type: &str,
    encoder_type: &str,
    forms: &Forms,
    parent: &syn::Ident,
    ty: &TokenStream2,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let inner_encoder = field_encoder(ty, forms, false, encoder_type, "", parent)?;
    Ok(quote! {::std::format!("\
{encoder_type} : {prefix} -> Json.Encode.Value
{encoder_type} {binders}({elm_type} inner) =
    ({inner_encoder}) inner
",
        prefix = #prefix,
        binders = #binders,
        elm_type = #elm_type,
        encoder_type = #encoder_type,
        inner_encoder = #inner_encoder,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// struct Tuple(i32, i32);
/// "[0,0]"
fn struct_tuple(
    elm_type: &str,
    encoder_type: &str,
    forms: &Forms,
    parent: &syn::Ident,
    fields: &[super::PositionalField],
) -> syn::Result<TokenStream2> {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let params: Vec<usize> = (0..fields.len()).collect();
    let list = positional_json_list(fields, forms, parent, encoder_type, true)?;

    Ok(quote! {::std::format!("\
{encoder_type} : {prefix} -> Json.Encode.Value
{encoder_type} {binders}({elm_type} {params}) =
    Json.Encode.list identity
        {list}
",
        prefix = #prefix,
        binders = #binders,
        elm_type = #elm_type,
        encoder_type = #encoder_type,
        params = (&[#(::std::format!("t{idx}",
                idx = #params)),*
            ]
        ).join(" "),
        list = #list,
    )})
}

/// JSON array elements. `skip_serializing_if` drops an element, so the list is shorter.
fn positional_json_list(
    fields: &[super::PositionalField],
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
    lazy_fields: bool,
) -> syn::Result<TokenStream2> {
    let conditional = fields
        .iter()
        .any(|field| !field.codec.skip_serializing && field.codec.is_conditional());
    let mut pieces = Vec::new();
    for (idx, field) in fields.iter().enumerate() {
        if field.codec.skip_serializing {
            continue;
        }
        let encoder = field_encoder(
            &field.ty,
            forms,
            lazy_fields && field.lazy,
            parent_encoder,
            "",
            parent,
        )?;
        if !conditional {
            pieces.push(quote! {
                ::std::format!("({encoder}) t{idx}", encoder = #encoder, idx = #idx)
            });
            continue;
        }
        let piece = positional_maybe(idx, field);
        pieces.push(quote! {
            ::std::format!(#piece, encoder = #encoder)
        });
    }
    if conditional {
        Ok(quote! {
            ::std::format!(
                "(List.filterMap identity [ {pieces} ])",
                pieces = (&[#(#pieces),*]).join(", "),
            )
        })
    } else {
        Ok(quote! {
            ::std::format!("[ {pieces} ]", pieces = (&[#(#pieces),*]).join(", "))
        })
    }
}

fn positional_maybe(idx: usize, field: &super::PositionalField) -> String {
    #[cfg(feature = "serde")]
    {
        use super::attributes::serde::SkipIf;
        return match field.codec.skip_if {
            Some(SkipIf::OptionIsNone) => {
                format!("(Maybe.map (\\_ -> ({{encoder}}) t{idx}) t{idx})")
            }
            Some(SkipIf::VecIsEmpty) => {
                format!("(if List.isEmpty t{idx} then Nothing else Just (({{encoder}}) t{idx}))")
            }
            Some(SkipIf::StringIsEmpty) => {
                format!("(if String.isEmpty t{idx} then Nothing else Just (({{encoder}}) t{idx}))")
            }
            None => format!("Just (({{encoder}}) t{idx})"),
        };
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = field;
        format!("Just (({{encoder}}) t{idx})")
    }
}

/// #[derive(Deserialize, Serialize)]
/// struct Struct {
///     a: i32,
/// };
/// "{\"a\":0}"
fn struct_named(
    elm_type: &str,
    encoder_type: &str,
    fields_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
    fields: &[StructField],
    recursive: bool,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let mut pieces = Vec::new();
    for field in fields {
        pieces.push(field_list_piece(
            field,
            "struct.",
            encoder_type,
            fields_name,
            forms,
            parent,
        )?);
    }
    let (pattern, value) = if recursive {
        (format!("({elm_type} struct)"), "value")
    } else {
        ("struct".to_string(), "struct")
    };
    Ok(quote! {::std::format!("\
{fields_name} : {prefix} -> List (String, Json.Encode.Value)
{fields_name} {binders}{pattern} =
    List.concat
        [ {pieces}
        ]

{encoder_type} : {prefix} -> Json.Encode.Value
{encoder_type} {binders}{value} =
    Json.Encode.object ({fields_name} {binders}{value})
",
        prefix = #prefix,
        binders = #binders,
        encoder_type = #encoder_type,
        fields_name = #fields_name,
        pattern = #pattern,
        value = #value,
        pieces = (&[#(#pieces),*]).join("\n        , "),
    )})
}

/// One field's contribution to a JSON object, as an Elm `List (String, Value)`.
fn field_list_piece(
    field: &StructField,
    access_prefix: &str,
    parent_encoder: &str,
    fields_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
) -> syn::Result<TokenStream2> {
    if field.codec.skip_serializing {
        return Ok(quote! { ::std::string::String::from("[]") });
    }
    let access = format!("{access_prefix}{}", field.name_elm());
    if field.flatten {
        let ty = &field.ty;
        return Ok(quote! {
            ::std::format!(
                "(({encoder}) {access})",
                encoder = <#ty as ::rs2elm::ElmEncode>::object_fields_encoder()
                    .expect("#[serde(flatten)] requires a type whose JSON encoding is an object"),
                access = #access,
            )
        });
    }
    let json_name = field.name_encode();
    #[cfg(feature = "serde")]
    {
        use super::attributes::serde::SkipIf;
        match field.codec.skip_if {
            Some(SkipIf::OptionIsNone) => {
                let inner = field
                    .skip_if_inner
                    .as_ref()
                    .expect("Option::is_none stores the inner type");
                let encoder = encoder_expr(
                    inner,
                    field.lazy,
                    parent_encoder,
                    fields_name,
                    forms,
                    parent,
                )?;
                return Ok(quote! {
                    ::std::format!(
                        "Maybe.withDefault [] (Maybe.map (\\v -> [ ( \"{json_name}\", ({encoder}) v ) ]) {access})",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            Some(SkipIf::VecIsEmpty) | Some(SkipIf::StringIsEmpty) => {}
            None => {}
        }
    }
    let ty = &field.ty;
    let encoder = encoder_expr(ty, field.lazy, parent_encoder, fields_name, forms, parent)?;
    #[cfg(feature = "serde")]
    {
        use super::attributes::serde::SkipIf;
        match field.codec.skip_if {
            Some(SkipIf::VecIsEmpty) => {
                return Ok(quote! {
                    ::std::format!(
                        "if List.isEmpty {access} then [] else [ ( \"{json_name}\", ({encoder}) {access} ) ]",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            Some(SkipIf::StringIsEmpty) => {
                return Ok(quote! {
                    ::std::format!(
                        "if String.isEmpty {access} then [] else [ ( \"{json_name}\", ({encoder}) {access} ) ]",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            Some(SkipIf::OptionIsNone) | None => {}
        }
    }
    Ok(quote! {
        ::std::format!(
            "[ ( \"{json_name}\", ({encoder}) {access} ) ]",
            json_name = #json_name,
            encoder = #encoder,
            access = #access,
        )
    })
}

fn field_encoder(
    ty: &TokenStream2,
    forms: &Forms,
    lazy: bool,
    parent_encoder: &str,
    fields_name: &str,
    parent: &syn::Ident,
) -> syn::Result<TokenStream2> {
    encoder_expr(ty, lazy, parent_encoder, fields_name, forms, parent)
}

/// Encoder expression for a field. A direct cycle calls this fields function, which is already
/// in scope, instead of the value encoder defined below it.
fn encoder_expr(
    ty: &TokenStream2,
    lazy: bool,
    parent_encoder: &str,
    fields_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
) -> syn::Result<TokenStream2> {
    let base = polymorph::encoder_expr(
        ty,
        &Ctx {
            params: &forms.params,
            rewrite_parent: if lazy { Some(parent_encoder) } else { None },
            parent_ident: Some(&parent.to_string()),
        },
    )?;
    if !lazy {
        return Ok(base);
    }
    let to = if forms.encode_binders.is_empty() {
        quote! { ::std::format!("(Json.Encode.object << {})", #fields_name) }
    } else {
        let to = format!(
            "(\\value -> Json.Encode.object ({fields_name} {}value))",
            forms.encode_binders
        );
        quote! { ::std::string::String::from(#to) }
    };
    Ok(quote! {
        {
            let encoder = #base;
            let from = #parent_encoder;
            let to = #to;
            let mut rewritten = ::std::string::String::new();
            let mut rest = encoder.as_str();
            while let ::std::option::Option::Some(index) = rest.find(from) {
                let after = index + from.len();
                let starts = index == 0
                    || {
                        let prev = rest[..index].chars().next_back().unwrap();
                        !prev.is_ascii_alphanumeric() && prev != '_'
                    };
                let ends = match rest[after..].chars().next() {
                    ::std::option::Option::None => true,
                    ::std::option::Option::Some(ch) => !ch.is_ascii_alphanumeric() && ch != '_',
                };
                if starts && ends {
                    rewritten.push_str(&rest[..index]);
                    rewritten.push_str(&to);
                } else {
                    rewritten.push_str(&rest[..after]);
                }
                rest = &rest[after..];
            }
            rewritten.push_str(rest);
            rewritten
        }
    })
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
    encoder_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
    variants: Vec<EnumVariant>,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let mut encoders = vec![];
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_encode = variant.name_encode();

        let encoder = match &variant.variant {
            EnumVariantKind::Unit => enum_variant_unit_external(&elm_name, &elm_name_encode),
            EnumVariantKind::Newtype(inner) => enum_variant_newtype_external(
                &elm_name,
                &elm_name_encode,
                &inner.ty,
                forms,
                parent,
                encoder_name,
            )?,
            EnumVariantKind::Tuple(types) => enum_variant_tuple_external(
                &elm_name,
                &elm_name_encode,
                types,
                forms,
                parent,
                encoder_name,
            )?,
            EnumVariantKind::Struct(fields) => enum_variant_struct_external(
                &elm_name,
                &elm_name_encode,
                fields,
                container_attributes,
                forms,
                parent,
                encoder_name,
            )?,
        };
        encoders.push(encoder);
    }

    Ok(quote! {::std::format!("\
{encoder_name} : {prefix} -> Json.Encode.Value
{encoder_name} {binders}enum =
    case enum of
        {encoders}",
        encoder_name = #encoder_name,
        prefix = #prefix,
        binders = #binders,
        encoders = (
            &[
                #(#encoders),*
            ]
        ).join("\n        ")
    )})
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
    encoder_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
    variants: Vec<EnumVariant>,
    tag: &str,
    fields_name: &str,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let mut arms = vec![];
    for variant in variants {
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_encode = variant.name_encode();

        let arm = match &variant.variant {
            EnumVariantKind::Unit => quote! {::std::format!("\
{variant_name} ->
            [ ( \"{tag}\", Json.Encode.string \"{variant_name_encode}\" ) ]",
                variant_name = #elm_name,
                tag = #tag,
                variant_name_encode = #elm_name_encode,
            )},
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
                let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
                let tag_piece =
                    format!("[ ( \"{tag}\", Json.Encode.string \"{elm_name_encode}\" ) ]");
                let mut pieces = vec![quote! { ::std::string::String::from(#tag_piece) }];
                for field in fields {
                    pieces.push(field_list_piece(
                        field,
                        "",
                        encoder_name,
                        fields_name,
                        forms,
                        parent,
                    )?);
                }
                quote! {::std::format!("\
{variant_name} {{ {fields} }} ->
            List.concat
                [ {pieces}
                ]",
                    variant_name = #elm_name,
                    fields = (&[#(#field_names),*]).join(", "),
                    pieces = (&[#(#pieces),*]).join("\n                , "),
                )}
            }
        };
        arms.push(arm);
    }

    Ok(quote! {::std::format!("\
{fields_name} : {prefix} -> List (String, Json.Encode.Value)
{fields_name} {binders}enum =
    case enum of
        {arms}

{encoder_name} : {prefix} -> Json.Encode.Value
{encoder_name} {binders}enum =
    Json.Encode.object ({fields_name} {binders}enum)
",
        fields_name = #fields_name,
        encoder_name = #encoder_name,
        prefix = #prefix,
        binders = #binders,
        arms = (&[#(#arms),*]).join("\n        "),
    )})
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
    encoder_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
    variants: Vec<EnumVariant>,
    tag: &str,
    content: &str,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let mut encoders = vec![];
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let elm_name_encode = variant.name_encode();

        let encoder = match &variant.variant {
            EnumVariantKind::Unit => {
                enum_variant_unit_internal_or_adjacent(tag, &elm_name, &elm_name_encode)
            }
            EnumVariantKind::Newtype(inner) => enum_variant_newtype_adjacent(
                tag,
                content,
                &elm_name,
                &elm_name_encode,
                &inner.ty,
                forms,
                parent,
                encoder_name,
            )?,
            EnumVariantKind::Tuple(types) => enum_variant_tuple_adjacent(
                tag,
                content,
                &elm_name,
                &elm_name_encode,
                types,
                forms,
                parent,
                encoder_name,
            )?,
            EnumVariantKind::Struct(fields) => enum_variant_struct_adjacent(
                tag,
                content,
                &elm_name,
                &elm_name_encode,
                fields,
                container_attributes,
                forms,
                parent,
                encoder_name,
            )?,
        };
        encoders.push(encoder);
    }

    Ok(quote! {::std::format!("\
{encoder_name} : {prefix} -> Json.Encode.Value
{encoder_name} {binders}enum =
    case enum of
        {encoders}",
        encoder_name = #encoder_name,
        prefix = #prefix,
        binders = #binders,
        encoders = (&[
                #(#encoders),*
        ]).join("\n        ")
    )})
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
    encoder_name: &str,
    forms: &Forms,
    parent: &syn::Ident,
    variants: Vec<EnumVariant>,
    container_attributes: &ContainerAttributes,
) -> syn::Result<TokenStream2> {
    let prefix = &forms.encode_prefix;
    let binders = &forms.encode_binders;
    let mut encoders = vec![];
    for variant in variants {
        #[cfg(feature = "serde")]
        if variant.serde_attributes.skip {
            continue;
        }

        let elm_name = variant.name_elm();
        let encoder = match &variant.variant {
            EnumVariantKind::Unit => enum_variant_unit_untagged(&elm_name),
            EnumVariantKind::Newtype(inner) => {
                enum_variant_newtype_untagged(&elm_name, &inner.ty, forms, parent, encoder_name)?
            }
            EnumVariantKind::Tuple(types) => {
                enum_variant_tuple_untagged(&elm_name, types, forms, parent, encoder_name)?
            }
            EnumVariantKind::Struct(fields) => enum_variant_struct_untagged(
                &elm_name,
                fields,
                container_attributes,
                forms,
                parent,
                encoder_name,
            )?,
        };
        encoders.push(encoder);
    }

    Ok(quote! {::std::format!("\
{encoder_name} : {prefix} -> Json.Encode.Value
{encoder_name} {binders}enum =
    case enum of
        {encoders}",
        encoder_name = #encoder_name,
        prefix = #prefix,
        binders = #binders,
        encoders = (
            &[
                #(#encoders),*
            ]
        ).join("\n        ")
    )})
}

// =================
// external variants
// =================

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Unit,
/// }
/// "\"Unit\""
fn enum_variant_unit_external(variant_name: &str, variant_name_encode: &str) -> TokenStream2 {
    quote! {::std::format!("\
    {variant_name} ->
            Json.Encode.string \"{variant_name_encode}\"",
        variant_name = #variant_name,
        variant_name_encode = #variant_name_encode,
    )}
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Newtype(i32),
/// }
/// "{\"Newtype\":0}"
fn enum_variant_newtype_external(
    variant_name: &str,
    variant_name_encode: &str,
    inner_type: &TokenStream2,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let encoder = field_encoder(inner_type, forms, false, parent_encoder, "", parent)?;
    Ok(quote! { ::std::format!("\
{variant_name} inner ->
            Json.Encode.object [ ( \"{variant_name_encode}\", ({encoder}) inner ) ]",
        variant_name = #variant_name,
        variant_name_encode = #variant_name_encode,
        encoder = #encoder,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Tuple(i32, i32),
/// }
/// "{\"Tuple\":[0,0]}"
fn enum_variant_tuple_external(
    variant_name: &str,
    variant_name_encode: &str,
    fields: &[super::PositionalField],
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let idx: Vec<usize> = (0..fields.len()).collect();
    let list = positional_json_list(fields, forms, parent, parent_encoder, false)?;
    Ok(quote! {::std::format!("\
{variant_name} {fields} ->
            Json.Encode.object [ ( \"{variant_name_encode}\", Json.Encode.list identity {list} ) ]",
        variant_name = #variant_name,
        fields = (
            &[
                #(::std::format!("t{}", #idx)
                ),*
            ]
        ).join(" "),
        variant_name_encode = #variant_name_encode,
        list = #list,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// enum External {
///     Struct { a: i32 },
/// }
/// "{\"Struct\":{\"a\":0}}"
fn enum_variant_struct_external(
    variant_name: &str,
    variant_name_encode: &str,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let object = json_object(fields, "", None, forms, parent, parent_encoder)?;

    Ok(quote! {::std::format!("\
{variant_name} {{ {fields} }} ->
            Json.Encode.object [ ( \"{variant_name_encode}\", Json.Encode.object {object} ) ]",
        variant_name = #variant_name,
        variant_name_encode = #variant_name_encode,
        fields = (&[#(#field_names),*]).join(", "),
        object = #object,
    )})
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
fn enum_variant_unit_internal_or_adjacent(
    tag: &str,
    variant_name: &str,
    variant_name_encode: &str,
) -> TokenStream2 {
    quote! {::std::format!("\
{variant_name} ->
            Json.Encode.object [ ( \"{tag}\", Json.Encode.string \"{variant_name_encode}\" ) ]",
        variant_name = #variant_name,
        tag = #tag,
        variant_name_encode = #variant_name_encode,
    )}
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
    tag: &str,
    content: &str,
    variant_name: &str,
    variant_name_encode: &str,
    inner_type: &TokenStream2,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let encoder = field_encoder(inner_type, forms, false, parent_encoder, "", parent)?;
    Ok(quote! { ::std::format!("\
    {variant_name} inner ->
            Json.Encode.object [ ( \"{tag}\", Json.Encode.string \"{variant_name_encode}\"), ( \"{content}\", ({encoder}) inner ) ]",
        variant_name = #variant_name,
        tag = #tag,
        content = #content,
        variant_name_encode = #variant_name_encode,
        encoder = #encoder,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Tuple(i32, i32),
/// }
/// "{\"t\":\"Tuple\",\"c\":[0,0]}"
#[cfg(feature = "serde")]
fn enum_variant_tuple_adjacent(
    tag: &str,
    content: &str,
    variant_name: &str,
    variant_name_encode: &str,
    fields: &[super::PositionalField],
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let idx: Vec<usize> = (0..fields.len()).collect();
    let list = positional_json_list(fields, forms, parent, parent_encoder, false)?;

    Ok(quote! { ::std::format!("\
    {variant_name} {params} ->
            Json.Encode.object [ ( \"{tag}\", Json.Encode.string \"{variant_name_encode}\"), ( \"{content}\", Json.Encode.list identity {list} ) ]",
        variant_name = #variant_name,
        params = (
            &[
                #(::std::format!("t{}", #idx)
                ),*
            ]
        ).join(" "),
        tag = #tag,
        content = #content,
        variant_name_encode = #variant_name_encode,
        list = #list,
    )})
}

/// #[derive(Deserialize, Serialize)]
/// #[serde(tag = "t", content = "c")]
/// enum Adjacent {
///     Struct { a: i32 },
/// }
/// "{\"t\":\"Struct\",\"c\":{\"a\":0}}"
#[cfg(feature = "serde")]
fn enum_variant_struct_adjacent(
    tag: &str,
    content: &str,
    variant_name: &str,
    variant_name_encode: &str,
    fields: &[StructField],
    container_attributes: &ContainerAttributes,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let object = json_object(fields, "", None, forms, parent, parent_encoder)?;

    Ok(quote! { ::std::format!("\
    {variant_name} {{ {fields} }} ->
            Json.Encode.object [ ( \"{tag}\", Json.Encode.string \"{variant_name_encode}\"), ( \"{content}\", Json.Encode.object {object} ) ]",
        variant_name = #variant_name,
        fields = (&[#(#field_names),*]).join(", "),
        tag = #tag,
        content = #content,
        variant_name_encode = #variant_name_encode,
        object = #object,
    )})
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
{variant_name} ->
            Json.Encode.null",
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
    inner: &TokenStream2,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let encoder = field_encoder(inner, forms, false, parent_encoder, "", parent)?;
    Ok(quote! {::std::format!("\
{variant_name} inner ->
            ({encoder}) inner",
        variant_name = #variant_name,
        encoder = #encoder,
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
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let idx: Vec<usize> = (0..fields.len()).collect();
    let list = positional_json_list(fields, forms, parent, parent_encoder, false)?;
    Ok(quote! {::std::format!("\
{variant_name} {params} ->
            Json.Encode.list identity
            {list}",
        variant_name = #variant_name,
        params = (
            &[
                #(::std::format!("t{idx}",
                    idx = #idx
                )),*
            ]
        ).join(" "),
        list = #list,
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
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let _ = container_attributes;
    let field_names: Vec<_> = fields.iter().map(|field| field.name_elm()).collect();
    let object = json_object(fields, "", None, forms, parent, parent_encoder)?;
    Ok(quote! {::std::format!("\
{variant_name} {{ {fields} }} ->
            Json.Encode.object {object}",
        variant_name = #variant_name,
        fields = (&[#(#field_names),*]).join(", "),
        object = #object,
    )})
}

fn json_object(
    fields: &[StructField],
    access_prefix: &str,
    tag: Option<(&str, &str)>,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    if fields.iter().any(|field| field.flatten) {
        return flat_json_object(fields, access_prefix, tag, forms, parent, parent_encoder);
    }
    let conditional = fields
        .iter()
        .any(|field| !field.codec.skip_serializing && field.codec.is_conditional());
    let mut pieces = Vec::new();
    if let Some((tag_key, variant_name)) = tag {
        let piece = if conditional {
            format!("Just ( \"{tag_key}\", Json.Encode.string \"{variant_name}\" )")
        } else {
            format!("( \"{tag_key}\", Json.Encode.string \"{variant_name}\" )")
        };
        pieces.push(quote! { ::std::string::String::from(#piece) });
    }
    for field in fields {
        if field.codec.skip_serializing {
            continue;
        }
        let json_name = field.name_encode();
        let access = format!("{access_prefix}{}", field.name_elm());
        if conditional {
            pieces.push(conditional_pair(
                field,
                &json_name,
                &access,
                forms,
                parent,
                parent_encoder,
            )?);
        } else {
            let encoder = field_encoder(&field.ty, forms, false, parent_encoder, "", parent)?;
            pieces.push(quote! {
                ::std::format!(
                    "( \"{json_name}\", ({encoder}) {access} )",
                    json_name = #json_name,
                    encoder = #encoder,
                    access = #access,
                )
            });
        }
    }
    if conditional {
        Ok(quote! {
            ::std::format!(
                "(List.filterMap identity [ {pieces} ])",
                pieces = (&[#(#pieces),*]).join(", ")
            )
        })
    } else {
        Ok(quote! {
            ::std::format!("[ {pieces} ]", pieces = (&[#(#pieces),*]).join(", "))
        })
    }
}

/// Flattened fields are already lists of pairs. Sibling fields become singleton lists.
fn flat_json_object(
    fields: &[StructField],
    access_prefix: &str,
    tag: Option<(&str, &str)>,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    let mut pieces = Vec::new();
    if let Some((tag_key, variant_name)) = tag {
        let piece = format!("[ ( \"{tag_key}\", Json.Encode.string \"{variant_name}\" ) ]");
        pieces.push(quote! { ::std::string::String::from(#piece) });
    }
    for field in fields {
        if field.flatten || field.codec.is_conditional() {
            pieces.push(field_list_piece(
                field,
                access_prefix,
                parent_encoder,
                "",
                forms,
                parent,
            )?);
            continue;
        }
        if field.codec.skip_serializing {
            pieces.push(quote! { ::std::string::String::from("[]") });
            continue;
        }
        let json_name = field.name_encode();
        let access = format!("{access_prefix}{}", field.name_elm());
        let encoder = field_encoder(&field.ty, forms, false, parent_encoder, "", parent)?;
        pieces.push(quote! {
            ::std::format!(
                "[ ( \"{json_name}\", ({encoder}) {access} ) ]",
                json_name = #json_name,
                encoder = #encoder,
                access = #access,
            )
        });
    }
    Ok(quote! {
        ::std::format!(
            "(List.concat [ {pieces} ])",
            pieces = (&[#(#pieces),*]).join(", ")
        )
    })
}

fn conditional_pair(
    field: &StructField,
    json_name: &str,
    access: &str,
    forms: &Forms,
    parent: &syn::Ident,
    parent_encoder: &str,
) -> syn::Result<TokenStream2> {
    #[cfg(feature = "serde")]
    {
        use super::attributes::serde::SkipIf;
        match field.codec.skip_if {
            Some(SkipIf::OptionIsNone) => {
                let inner = field
                    .skip_if_inner
                    .as_ref()
                    .expect("Option::is_none stores the inner type");
                let encoder = field_encoder(inner, forms, field.lazy, parent_encoder, "", parent)?;
                return Ok(quote! {
                    ::std::format!(
                        "Maybe.map (\\v -> ( \"{json_name}\", ({encoder}) v )) {access}",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            Some(SkipIf::VecIsEmpty) => {
                let encoder =
                    field_encoder(&field.ty, forms, field.lazy, parent_encoder, "", parent)?;
                return Ok(quote! {
                    ::std::format!(
                        "if List.isEmpty {access} then Nothing else Just ( \"{json_name}\", ({encoder}) {access} )",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            Some(SkipIf::StringIsEmpty) => {
                let encoder =
                    field_encoder(&field.ty, forms, field.lazy, parent_encoder, "", parent)?;
                return Ok(quote! {
                    ::std::format!(
                        "if String.isEmpty {access} then Nothing else Just ( \"{json_name}\", ({encoder}) {access} )",
                        json_name = #json_name,
                        encoder = #encoder,
                        access = #access,
                    )
                });
            }
            None => {}
        }
    }
    #[cfg(not(feature = "serde"))]
    let _ = field;
    let encoder = field_encoder(&field.ty, forms, false, parent_encoder, "", parent)?;
    Ok(quote! {
        ::std::format!(
            "Just ( \"{json_name}\", ({encoder}) {access} )",
            json_name = #json_name,
            encoder = #encoder,
            access = #access,
        )
    })
}
