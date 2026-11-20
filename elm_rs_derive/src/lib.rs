//! Derive macros for rs2elm.

#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    rust_2018_idioms,
    unused_must_use,
    unreachable_pub,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    single_use_lifetimes,
    rustdoc::broken_intra_doc_links
)]

mod attributes;
mod elm;
#[cfg(feature = "json")]
mod elm_decode;
#[cfg(feature = "json")]
mod elm_encode;
#[cfg(feature = "query")]
mod elm_query;
#[cfg(feature = "query")]
mod elm_query_field;
mod polymorph;

use self::attributes::{ContainerAttributes, FieldAttributes, VariantAttributes};
use heck::{ToLowerCamelCase, ToPascalCase};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote};
use std::borrow::Cow;
use syn::{
    Data, DataEnum, DeriveInput, Fields, FieldsNamed, Generics, Ident, Type, Variant,
    punctuated::Punctuated, spanned::Spanned,
};
use tracing::instrument;

/// Derive `Elm`.
#[proc_macro_derive(Elm, attributes(rs2elm))]
pub fn derive_elm(input: TokenStream) -> TokenStream {
    elm::derive(input)
}

/// Derive `ElmEncode`.
#[cfg(feature = "json")]
#[proc_macro_derive(ElmEncode, attributes(rs2elm))]
pub fn derive_elm_serialize(input: TokenStream) -> TokenStream {
    elm_encode::derive(input)
}

/// Derive `ElmDecode`.
#[cfg(feature = "json")]
#[proc_macro_derive(ElmDecode, attributes(rs2elm))]
pub fn derive_elm_deserialize(input: TokenStream) -> TokenStream {
    elm_decode::derive(input)
}

/// Derive `ElmQuery`.
#[cfg(feature = "query")]
#[proc_macro_derive(ElmQuery, attributes(rs2elm))]
pub fn derive_elm_query(input: TokenStream) -> TokenStream {
    elm_query::derive(input)
}

/// Derive `ElmQueryField`.
#[cfg(feature = "query")]
#[proc_macro_derive(ElmQueryField, attributes(rs2elm))]
pub fn derive_elm_query_field(input: TokenStream) -> TokenStream {
    elm_query_field::derive(input)
}

/// Intermediate representation of the derive input for more convenient handling.
struct Intermediate {
    ident: Ident,
    elm_type: String,
    generics: Generics,
    generics_without_bounds: Generics,
    type_info: TypeInfo,
    container_attributes: ContainerAttributes,
}

impl Intermediate {
    /// Parse the derive input into the intermediate representation.
    #[instrument(skip(input), fields(ty = %input.ident))]
    fn parse(input: DeriveInput) -> syn::Result<Self> {
        let container_attributes = ContainerAttributes::parse(&input.attrs)?;
        let type_info = TypeInfo::parse(input.data, &container_attributes, &input.ident)?;

        let elm_type = container_attributes
            .elm
            .rename
            .clone()
            .unwrap_or_else(|| input.ident.to_string().to_pascal_case());
        check_elm_name(&elm_type, input.ident.span())?;
        let mut generics_without_bounds = input.generics.clone();
        for p in generics_without_bounds.type_params_mut() {
            p.bounds = Punctuated::default();
        }
        Ok(Self {
            ident: input.ident,
            elm_type,
            generics: input.generics,
            generics_without_bounds,
            type_info,
            container_attributes,
        })
    }
}

enum TypeInfo {
    // struct S;
    Unit,
    // struct S(String);
    Newtype(PositionalField),
    // struct S(String, u32);
    Tuple(Vec<PositionalField>),
    // struct S {
    //     s: String,
    // }
    Struct(Vec<StructField>),
    // enum E {
    //     Variant,
    // }
    Enum {
        variants: Vec<EnumVariant>,
        #[cfg(feature = "serde")]
        representation: attributes::serde::EnumRepresentation,
    },
}

impl TypeInfo {
    pub(crate) fn parse(
        data: Data,
        container_attributes: &ContainerAttributes,
        parent: &Ident,
    ) -> syn::Result<Self> {
        let type_info = match data {
            Data::Struct(data_struct) => match data_struct.fields {
                Fields::Unit => TypeInfo::Unit,
                Fields::Unnamed(unnamed) => {
                    let declared = unnamed.unnamed.len();
                    let fields =
                        parse_positional_fields(unnamed.unnamed, container_attributes, parent)?;
                    if declared == 1 {
                        TypeInfo::Newtype(fields.into_iter().next().ok_or_else(|| {
                            syn::Error::new(
                                Span::call_site(),
                                "cannot skip the only field of a newtype",
                            )
                        })?)
                    } else {
                        TypeInfo::Tuple(fields)
                    }
                }
                Fields::Named(named) => {
                    #[cfg(not(feature = "serde"))]
                    let transparent = false;
                    #[cfg(feature = "serde")]
                    let transparent = container_attributes.serde.transparent;
                    if transparent && named.named.len() == 1 {
                        let field = named.named.into_iter().next().unwrap();
                        let span = field.span();
                        TypeInfo::Newtype(
                            parse_positional(field, container_attributes, parent)?.ok_or_else(
                                || {
                                    syn::Error::new(
                                        span,
                                        "cannot skip the only field of a transparent struct",
                                    )
                                },
                            )?,
                        )
                    } else {
                        TypeInfo::Struct(StructField::parse(
                            named,
                            container_attributes,
                            None,
                            parent,
                        )?)
                    }
                }
            },
            Data::Enum(DataEnum { variants, .. }) => {
                if variants.is_empty() {
                    return Err(syn::Error::new(
                        variants.span(),
                        "empty enums are not supported",
                    ));
                }
                let variants = variants
                    .into_iter()
                    .map(|variant| EnumVariant::parse(variant, container_attributes, parent))
                    .collect::<Result<Vec<_>, _>>()?;
                check_unique_names(
                    variants
                        .iter()
                        .map(|variant| (variant.name_elm().into_owned(), variant.span)),
                    "variant",
                )?;

                TypeInfo::Enum {
                    #[cfg(feature = "serde")]
                    representation: container_attributes.serde.enum_representation.clone(),
                    variants,
                }
            }
            Data::Union(union) => {
                return Err(syn::Error::new(
                    union.union_token.span(),
                    "unions are not supported",
                ));
            }
        };
        Ok(type_info)
    }
}

#[derive(Clone, Copy, Default)]
struct FieldCodec {
    skip_serializing: bool,
    skip_deserializing: bool,
    #[cfg(feature = "serde")]
    skip_if: Option<attributes::serde::SkipIf>,
    #[cfg(feature = "serde")]
    missing: Option<ElmMissing>,
}

impl FieldCodec {
    fn is_conditional(self) -> bool {
        #[cfg(feature = "serde")]
        {
            self.skip_if.is_some()
        }
        #[cfg(not(feature = "serde"))]
        {
            false
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ElmMissing {
    Nothing,
    Unit,
}

struct StructField {
    ident: Ident,
    elm_name: String,
    encode_name: String,
    decode_name: String,
    #[cfg(feature = "serde")]
    decode_aliases: Vec<String>,
    lazy: bool,
    index: bool,
    flatten: bool,
    ty: TokenStream2,
    codec: FieldCodec,
    #[cfg(feature = "serde")]
    skip_if_inner: Option<TokenStream2>,
}

impl StructField {
    /// The name in the Elm type definition.
    fn name_elm(&self) -> String {
        self.elm_name.clone()
    }

    #[cfg(any(feature = "json", feature = "query"))]
    fn name_encode(&self) -> String {
        self.encode_name.clone()
    }

    #[cfg(any(feature = "json", feature = "query"))]
    fn name_decode(&self) -> String {
        self.decode_name.clone()
    }

    #[cfg(feature = "serde")]
    fn decode_names(&self) -> Vec<String> {
        let mut names = vec![self.name_decode()];
        for alias in &self.decode_aliases {
            if !names.contains(alias) {
                names.push(alias.clone());
            }
        }
        names
    }

    fn parse(
        fields: FieldsNamed,
        container: &ContainerAttributes,
        variant: Option<&VariantAttributes>,
        parent: &Ident,
    ) -> syn::Result<Vec<Self>> {
        let mut parsed = Vec::new();
        for field in fields.named {
            let attributes = FieldAttributes::parse(&field.attrs)?;
            #[cfg(feature = "serde")]
            if attributes.serde.flatten && cannot_flatten(&field.ty) {
                return Err(syn::Error::new(
                    field.span(),
                    "cannot flatten this type; it does not encode as a JSON object",
                ));
            }
            #[cfg(feature = "serde")]
            if attributes.serde.skip {
                continue;
            }
            let ident = field.ident.unwrap();
            let elm_name = attributes
                .elm
                .rename
                .clone()
                .unwrap_or_else(|| ident_string(&ident).to_lower_camel_case());
            check_elm_name(&elm_name, ident.span())?;
            let (encode_name, decode_name) =
                json_field_names(&ident, &attributes, container, variant);
            #[cfg(feature = "serde")]
            let decode_aliases = attributes.serde.aliases.clone();
            let (codec, skip_if_inner) =
                field_codec(&field.ty, ident.span(), &attributes, container)?;
            #[cfg(not(feature = "serde"))]
            let _ = &skip_if_inner;
            parsed.push(StructField {
                ident,
                elm_name,
                encode_name,
                decode_name,
                #[cfg(feature = "serde")]
                decode_aliases,
                lazy: attributes.elm.lazy || is_direct_cycle(&field.ty, parent),
                index: attributes.elm.index,
                #[cfg(feature = "serde")]
                flatten: attributes.serde.flatten,
                #[cfg(not(feature = "serde"))]
                flatten: false,
                ty: field.ty.to_token_stream(),
                codec,
                #[cfg(feature = "serde")]
                skip_if_inner,
            });
        }
        check_unique_names(
            parsed
                .iter()
                .map(|field| (field.elm_name.clone(), field.ident.span())),
            "field",
        )?;
        Ok(parsed)
    }
}

struct EnumVariant {
    #[allow(dead_code)]
    ident: Ident,
    elm_name: String,
    encode_name: String,
    decode_name: String,
    #[cfg(feature = "serde")]
    decode_aliases: Vec<String>,
    variant: EnumVariantKind,
    span: Span,
    #[cfg(feature = "serde")]
    serde_attributes: attributes::serde::VariantAttributes,
}

impl EnumVariant {
    /// The name in the Elm type definition.
    fn name_elm(&'_ self) -> Cow<'_, str> {
        Cow::Borrowed(&self.elm_name)
    }

    #[cfg(any(feature = "json", feature = "query"))]
    fn name_encode(&self) -> String {
        self.encode_name.clone()
    }

    #[cfg(any(feature = "json", feature = "query"))]
    fn name_decode(&self) -> String {
        self.decode_name.clone()
    }

    #[cfg(feature = "serde")]
    fn decode_names(&self) -> Vec<String> {
        let mut names = vec![self.name_decode()];
        for alias in &self.decode_aliases {
            if !names.contains(alias) {
                names.push(alias.clone());
            }
        }
        names
    }

    fn parse(
        variant: Variant,
        container: &ContainerAttributes,
        parent: &Ident,
    ) -> syn::Result<Self> {
        let span = variant.span();
        let variant_attributes = VariantAttributes::parse(&variant.attrs)?;
        let elm_name = variant_attributes
            .elm
            .rename
            .clone()
            .unwrap_or_else(|| ident_string(&variant.ident).to_pascal_case());
        check_elm_name(&elm_name, span)?;
        let (encode_name, decode_name) =
            json_variant_names(&variant.ident, &variant_attributes, container);
        #[cfg(feature = "serde")]
        let decode_aliases = variant_attributes.serde.aliases.clone();
        let variant_kind = match variant.fields {
            Fields::Unit => EnumVariantKind::Unit,
            Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                let span = unnamed.span();
                let fields = parse_positional_fields(unnamed.unnamed, container, parent)?;
                EnumVariantKind::Newtype(fields.into_iter().next().ok_or_else(|| {
                    syn::Error::new(span, "cannot skip the only field of a newtype variant")
                })?)
            }
            Fields::Unnamed(unnamed) => {
                EnumVariantKind::Tuple(parse_positional_fields(unnamed.unnamed, container, parent)?)
            }
            Fields::Named(named) => EnumVariantKind::Struct(StructField::parse(
                named,
                container,
                Some(&variant_attributes),
                parent,
            )?),
        };
        let variant = EnumVariant {
            ident: variant.ident,
            elm_name,
            encode_name,
            decode_name,
            #[cfg(feature = "serde")]
            decode_aliases,
            variant: variant_kind,
            span,
            #[cfg(feature = "serde")]
            serde_attributes: variant_attributes.serde,
        };
        Ok(variant)
    }
}

enum EnumVariantKind {
    // Variant,
    // "Variant"
    Unit,
    // Variant(String),
    // {"Variant": "string"}
    Newtype(PositionalField),
    // Variant(String, u32),
    // {"Variant": []}
    // {"Variant": ["string", 0]}
    Tuple(Vec<PositionalField>),
    // Variant {
    //     s: String,
    // }
    // {}
    // {"s": "string"}
    Struct(Vec<StructField>),
}

fn field_codec(
    ty: &Type,
    span: Span,
    attributes: &FieldAttributes,
    container: &ContainerAttributes,
) -> syn::Result<(FieldCodec, Option<TokenStream2>)> {
    #[cfg(not(feature = "serde"))]
    {
        let _ = (ty, span, attributes, container);
        return Ok((FieldCodec::default(), None));
    }
    #[cfg(feature = "serde")]
    {
        use attributes::serde::SkipIf;

        let mut codec = FieldCodec::default();
        codec.skip_serializing = attributes.serde.skip_serializing;
        codec.skip_deserializing = attributes.serde.skip_deserializing;
        let mut skip_if_inner = None;
        if let Some(skip_if) = attributes.serde.skip_serializing_if {
            match skip_if {
                SkipIf::OptionIsNone => {
                    if !type_is_named(ty, "Option") {
                        return Err(syn::Error::new(
                            span,
                            "`Option::is_none` requires a field of type `Option<_>`",
                        ));
                    }
                    skip_if_inner = Some(generic_inner(ty)?);
                }
                SkipIf::VecIsEmpty => {
                    if !type_is_named(ty, "Vec") {
                        return Err(syn::Error::new(
                            span,
                            "`Vec::is_empty` requires a field of type `Vec<_>`",
                        ));
                    }
                }
                SkipIf::StringIsEmpty => {
                    if !type_is_named(ty, "String") {
                        return Err(syn::Error::new(
                            span,
                            "`String::is_empty` requires a field of type `String`",
                        ));
                    }
                }
            }
            codec.skip_if = Some(skip_if);
        }
        if attributes.serde.default || container.serde.default || codec.skip_deserializing {
            codec.missing = Some(elm_missing(ty).ok_or_else(|| {
                syn::Error::new(span, "the Elm side has no Rust `Default` value")
            })?);
        }
        Ok((codec, skip_if_inner))
    }
}

#[cfg(feature = "serde")]
fn cannot_flatten(ty: &Type) -> bool {
    match ty {
        Type::Tuple(_) | Type::Array(_) | Type::Slice(_) => true,
        Type::Path(path) => path.path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "String"
                    | "str"
                    | "bool"
                    | "char"
                    | "f32"
                    | "f64"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "Vec"
                    | "Option"
                    | "Path"
                    | "PathBuf"
                    | "IpAddr"
                    | "Ipv4Addr"
                    | "Ipv6Addr"
                    | "SocketAddr"
                    | "SocketAddrV4"
                    | "SocketAddrV6"
            )
        }),
        _ => false,
    }
}

fn type_is_named(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name),
        _ => false,
    }
}

fn elm_missing(ty: &Type) -> Option<ElmMissing> {
    if type_is_named(ty, "Option") {
        Some(ElmMissing::Nothing)
    } else if matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty()) {
        Some(ElmMissing::Unit)
    } else {
        None
    }
}

fn generic_inner(ty: &Type) -> syn::Result<TokenStream2> {
    let Type::Path(path) = ty else {
        return Err(syn::Error::new(ty.span(), "expected a generic type"));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(syn::Error::new(ty.span(), "expected a generic type"));
    };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return Err(syn::Error::new(ty.span(), "expected a generic type"));
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(inner)) => Ok(inner.to_token_stream()),
        _ => Err(syn::Error::new(ty.span(), "expected one type argument")),
    }
}

struct PositionalField {
    ty: TokenStream2,
    codec: FieldCodec,
    lazy: bool,
}

fn parse_positional_fields(
    fields: Punctuated<syn::Field, syn::token::Comma>,
    container: &ContainerAttributes,
    parent: &Ident,
) -> syn::Result<Vec<PositionalField>> {
    let mut parsed = Vec::new();
    #[cfg(feature = "serde")]
    let mut default_at = None;
    for (index, field) in fields.into_iter().enumerate() {
        #[cfg(feature = "serde")]
        {
            let attributes = FieldAttributes::parse(&field.attrs)?;
            // `skip` removes the element. `skip_deserializing` fills it and does not
            // start serde's "later tuple fields need default" rule.
            let ignored = attributes.serde.skip || attributes.serde.skip_deserializing;
            if !ignored {
                if let Some(previous) = default_at {
                    if !attributes.serde.default && !container.serde.default {
                        return Err(syn::Error::new(
                            field.span(),
                            format!(
                                "field must have #[serde(default)] because previous field {previous} has #[serde(default)]"
                            ),
                        ));
                    }
                }
            }
        }
        if let Some(parsed_field) = parse_positional(field, container, parent)? {
            #[cfg(feature = "serde")]
            if default_at.is_none()
                && parsed_field.codec.missing.is_some()
                && !parsed_field.codec.skip_deserializing
            {
                default_at = Some(index);
            }
            parsed.push(parsed_field);
        }
    }
    Ok(parsed)
}

fn parse_positional(
    field: syn::Field,
    container: &ContainerAttributes,
    parent: &Ident,
) -> syn::Result<Option<PositionalField>> {
    let attributes = FieldAttributes::parse(&field.attrs)?;
    #[cfg(feature = "serde")]
    if attributes.serde.flatten {
        return Err(syn::Error::new(
            field.span(),
            "flatten is only supported on named fields",
        ));
    }
    #[cfg(feature = "serde")]
    if attributes.serde.skip {
        return Ok(None);
    }
    let span = field.span();
    let lazy = attributes.elm.lazy || is_direct_cycle(&field.ty, parent);
    let (codec, _skip_if_inner) = field_codec(&field.ty, span, &attributes, container)?;
    Ok(Some(PositionalField {
        ty: field.ty.to_token_stream(),
        codec,
        lazy,
    }))
}

/// The field type is the parent, or the parent wrapped only in pointer, option, list, or cow layers.
fn is_direct_cycle(ty: &Type, parent: &Ident) -> bool {
    let mut current = ty;
    loop {
        if type_is_parent(current, parent) {
            return true;
        }
        match peel_wrapper(current) {
            Some(inner) => current = inner,
            None => return false,
        }
    }
}

fn type_is_parent(ty: &Type, parent: &Ident) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .is_some_and(|segment| &segment.ident == parent),
        _ => false,
    }
}

fn peel_wrapper(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(reference) => Some(&reference.elem),
        Type::Slice(slice) => Some(&slice.elem),
        Type::Array(array) => Some(&array.elem),
        Type::Path(path) if path.qself.is_none() => {
            let segment = path.path.segments.last()?;
            if !matches!(
                segment.ident.to_string().as_str(),
                "Box" | "Rc" | "Arc" | "Option" | "Vec" | "Cow"
            ) {
                return None;
            }
            let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                return None;
            };
            args.args.iter().rev().find_map(|arg| match arg {
                syn::GenericArgument::Type(inner) => Some(inner),
                _ => None,
            })
        }
        _ => None,
    }
}

fn dependency_types(type_info: &TypeInfo) -> Vec<TokenStream2> {
    let mut types = Vec::new();
    match type_info {
        TypeInfo::Unit => {}
        TypeInfo::Newtype(field) => types.push(field.ty.clone()),
        TypeInfo::Tuple(fields) => types.extend(fields.iter().map(|field| field.ty.clone())),
        TypeInfo::Struct(fields) => types.extend(fields.iter().map(|field| field.ty.clone())),
        TypeInfo::Enum { variants, .. } => {
            for variant in variants {
                match &variant.variant {
                    EnumVariantKind::Unit => {}
                    EnumVariantKind::Newtype(field) => types.push(field.ty.clone()),
                    EnumVariantKind::Tuple(fields) => {
                        types.extend(fields.iter().map(|field| field.ty.clone()))
                    }
                    EnumVariantKind::Struct(fields) => {
                        types.extend(fields.iter().map(|field| field.ty.clone()))
                    }
                }
            }
        }
    }
    types
}

fn contribute_calls(deps: &[TokenStream2], trait_path: TokenStream2, method: &str) -> TokenStream2 {
    let method = Ident::new(method, Span::call_site());
    let calls = deps.iter().map(|ty| {
        let trait_path = trait_path.clone();
        let method = method.clone();
        quote! { <#ty as #trait_path>::#method(module); }
    });
    quote! { #(#calls)* }
}

fn ident_string(ident: &Ident) -> String {
    let raw = ident.to_string();
    raw.strip_prefix("r#").unwrap_or(&raw).to_string()
}

fn is_elm_keyword(name: &str) -> bool {
    matches!(
        name,
        "if" | "then"
            | "else"
            | "case"
            | "of"
            | "let"
            | "in"
            | "type"
            | "module"
            | "where"
            | "import"
            | "exposing"
            | "as"
            | "alias"
            | "port"
            | "infix"
            | "infixl"
            | "infixr"
    )
}

fn check_elm_name(name: &str, span: Span) -> syn::Result<()> {
    if is_elm_keyword(name) {
        Err(syn::Error::new(
            span,
            format!("`{name}` is an Elm keyword. Rename it with #[rs2elm(rename = \"...\")]"),
        ))
    } else {
        Ok(())
    }
}

fn check_unique_names(names: impl Iterator<Item = (String, Span)>, kind: &str) -> syn::Result<()> {
    let mut seen = std::collections::HashMap::<String, Span>::new();
    for (name, span) in names {
        if let Some(previous) = seen.insert(name.clone(), span) {
            let mut error = syn::Error::new(
                span,
                format!(
                    "Elm {kind} `{name}` is already used. Rename it with #[rs2elm(rename = \"...\")]"
                ),
            );
            error.combine(syn::Error::new(
                previous,
                format!("`{name}` was first used here"),
            ));
            return Err(error);
        }
    }
    Ok(())
}

fn json_field_names(
    ident: &Ident,
    attributes: &FieldAttributes,
    container: &ContainerAttributes,
    variant: Option<&VariantAttributes>,
) -> (String, String) {
    #[cfg(feature = "serde")]
    {
        let plain = ident_string(ident);
        let encode = if let Some(rename) = attributes
            .serde
            .rename
            .as_ref()
            .or(attributes.serde.rename_deserialize.as_ref())
        {
            rename.clone()
        } else if let Some(variant) = variant {
            if let Some(rule) = variant
                .serde
                .rename_all
                .or(variant.serde.rename_all_deserialize)
            {
                rule.rename_ident(ident)
            } else if let Some(rule) = container
                .serde
                .rename_all_fields
                .or(container.serde.rename_all_fields_deserialize)
            {
                rule.rename_ident(ident)
            } else {
                plain.clone()
            }
        } else if let Some(rule) = container
            .serde
            .rename_all
            .or(container.serde.rename_all_deserialize)
        {
            rule.rename_ident(ident)
        } else {
            plain.clone()
        };
        let decode = if let Some(rename) = attributes
            .serde
            .rename
            .as_ref()
            .or(attributes.serde.rename_serialize.as_ref())
        {
            rename.clone()
        } else if let Some(variant) = variant {
            if let Some(rule) = variant
                .serde
                .rename_all
                .or(variant.serde.rename_all_serialize)
            {
                rule.rename_ident(ident)
            } else if let Some(rule) = container
                .serde
                .rename_all_fields
                .or(container.serde.rename_all_fields_serialize)
            {
                rule.rename_ident(ident)
            } else {
                plain
            }
        } else if let Some(rule) = container
            .serde
            .rename_all
            .or(container.serde.rename_all_serialize)
        {
            rule.rename_ident(ident)
        } else {
            plain
        };
        return (encode, decode);
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = (attributes, container, variant);
        let plain = ident_string(ident);
        (plain.clone(), plain)
    }
}

fn json_variant_names(
    ident: &Ident,
    attributes: &VariantAttributes,
    container: &ContainerAttributes,
) -> (String, String) {
    #[cfg(feature = "serde")]
    {
        let plain = ident_string(ident);
        let encode = if let Some(rename) = attributes
            .serde
            .rename
            .as_ref()
            .or(attributes.serde.rename_deserialize.as_ref())
        {
            rename.clone()
        } else if let Some(rule) = container
            .serde
            .rename_all
            .or(container.serde.rename_all_deserialize)
        {
            rule.rename_ident(ident)
        } else {
            plain.clone()
        };
        let decode = if let Some(rename) = attributes
            .serde
            .rename
            .as_ref()
            .or(attributes.serde.rename_serialize.as_ref())
        {
            rename.clone()
        } else if let Some(rule) = container
            .serde
            .rename_all
            .or(container.serde.rename_all_serialize)
        {
            rule.rename_ident(ident)
        } else {
            plain
        };
        return (encode, decode);
    }
    #[cfg(not(feature = "serde"))]
    {
        let _ = (attributes, container);
        let plain = ident_string(ident);
        (plain.clone(), plain)
    }
}
