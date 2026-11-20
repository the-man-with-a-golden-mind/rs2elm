//! Translate a Rust type into Elm when the surrounding type has type parameters.
//!
//! A type that does not mention those parameters keeps the existing trait call, so
//! non-generic codecs stay as they are. A type that mentions one is printed here,
//! with `T` as `a`, then `b`, then `c`.

use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{GenericArgument, Generics, Type, TypePath};

#[derive(Clone)]
pub(crate) struct Param {
    pub(crate) ident: String,
    pub(crate) var: String,
}

pub(crate) struct Forms {
    pub(crate) params: Vec<Param>,
    /// `Foo` or `Foo a b`.
    pub(crate) applied: String,
    /// Empty, or `(a -> Json.Encode.Value) -> `.
    pub(crate) encode_prefix: String,
    /// Empty, or `aEncoder `.
    pub(crate) encode_binders: String,
    /// `Json.Decode.Decoder Foo`, or `(Json.Decode.Decoder a) -> Json.Decode.Decoder (Foo a)`.
    pub(crate) decode_prefix: String,
    /// Empty, or `aDecoder `.
    pub(crate) decode_binders: String,
    /// Empty, or `(a -> String) -> `.
    pub(crate) query_prefix: String,
    /// Empty, or `aQuery `.
    pub(crate) query_binders: String,
}

impl Forms {
    pub(crate) fn from_generics(elm_type: &str, generics: &Generics) -> Self {
        let names = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
        let params = generics
            .type_params()
            .enumerate()
            .map(|(index, param)| {
                let var = names
                    .get(index)
                    .copied()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("t{index}"));
                Param {
                    ident: param.ident.to_string(),
                    var,
                }
            })
            .collect::<Vec<_>>();
        let applied = if params.is_empty() {
            elm_type.to_string()
        } else {
            format!(
                "{elm_type} {}",
                params
                    .iter()
                    .map(|param| param.var.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        };
        let encode_prefix = params
            .iter()
            .map(|param| format!("({} -> Json.Encode.Value) -> ", param.var))
            .collect::<String>()
            + &applied;
        let encode_binders = binder(&params, "Encoder");
        let decode_ann = if applied.contains(' ') {
            format!("Json.Decode.Decoder ({applied})")
        } else {
            format!("Json.Decode.Decoder {applied}")
        };
        let decode_prefix = params
            .iter()
            .map(|param| format!("(Json.Decode.Decoder {}) -> ", param.var))
            .collect::<String>()
            + &decode_ann;
        let decode_binders = binder(&params, "Decoder");
        let query_prefix = params
            .iter()
            .map(|param| format!("({} -> String) -> ", param.var))
            .collect::<String>()
            + &applied;
        let query_binders = binder(&params, "Query");
        Self {
            params,
            applied,
            encode_prefix,
            encode_binders,
            decode_prefix,
            decode_binders,
            query_prefix,
            query_binders,
        }
    }
}

/// `(name (arg) (arg))` so a codec with spaces stays one Elm argument.
pub(crate) fn applied_call(
    bare: &str,
    generics: &Generics,
    trait_path: TokenStream2,
    method: &str,
) -> TokenStream2 {
    let idents: Vec<_> = generics
        .type_params()
        .map(|param| param.ident.clone())
        .collect();
    if idents.is_empty() {
        return quote::quote! { ::std::convert::From::from(#bare) };
    }
    let method = syn::Ident::new(method, proc_macro2::Span::call_site());
    let calls = idents.iter().map(|ident| {
        let trait_path = trait_path.clone();
        let method = method.clone();
        quote::quote! { <#ident as #trait_path>::#method() }
    });
    let slots = std::iter::repeat("({})")
        .take(idents.len())
        .collect::<Vec<_>>()
        .join(" ");
    let template = format!("({{}} {slots})");
    quote::quote! { ::std::format!(#template, #bare, #(#calls),*) }
}

fn binder(params: &[Param], suffix: &str) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!(
            "{} ",
            params
                .iter()
                .map(|param| format!("{}{suffix}", param.var))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

pub(crate) struct Ctx<'a> {
    pub(crate) params: &'a [Param],
    /// When set, a direct cycle of this type is printed as the bare codec name so the
    /// existing lazy rewrite can point it at the fields function.
    pub(crate) rewrite_parent: Option<&'a str>,
    pub(crate) parent_ident: Option<&'a str>,
}

fn contains_param(ty: &Type, params: &[Param]) -> bool {
    match ty {
        Type::Path(path) => {
            path_is_param(path, params)
                || path
                    .path
                    .segments
                    .iter()
                    .any(|segment| match &segment.arguments {
                        syn::PathArguments::AngleBracketed(args) => {
                            args.args.iter().any(|arg| match arg {
                                GenericArgument::Type(inner) => contains_param(inner, params),
                                _ => false,
                            })
                        }
                        _ => false,
                    })
        }
        Type::Reference(reference) => contains_param(&reference.elem, params),
        Type::Slice(slice) => contains_param(&slice.elem, params),
        Type::Array(array) => contains_param(&array.elem, params),
        Type::Tuple(tuple) => tuple.elems.iter().any(|elem| contains_param(elem, params)),
        Type::Paren(inner) => contains_param(&inner.elem, params),
        Type::Group(inner) => contains_param(&inner.elem, params),
        _ => false,
    }
}

fn path_is_param(path: &TypePath, params: &[Param]) -> bool {
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return false;
    }
    let segment = &path.path.segments[0];
    segment.arguments.is_empty() && params.iter().any(|param| segment.ident == param.ident)
}

fn param_var<'a>(path: &TypePath, params: &'a [Param]) -> Option<&'a str> {
    if !path_is_param(path, params) {
        return None;
    }
    params
        .iter()
        .find(|param| path.path.segments[0].ident == param.ident)
        .map(|param| param.var.as_str())
}

pub(crate) fn elm_type_expr(ty: &TokenStream2, params: &[Param]) -> syn::Result<TokenStream2> {
    let parsed = parse_type(ty)?;
    // `Leaf<i32>::elm_type()` is the bare name `Leaf`. A field of that shape has to
    // print `Leaf Int`, or the generated alias does not compile.
    if !contains_param(&parsed, params) && !concrete_user_application(&parsed) {
        return Ok(quote_trait(ty, "::rs2elm::Elm", "elm_type"));
    }
    structural_elm(&parsed, params, ty.span())
}

fn concrete_user_application(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    let Some(segment) = path.path.segments.last() else {
        return false;
    };
    let Some(args) = angle_args(segment) else {
        return false;
    };
    if !args
        .iter()
        .any(|arg| matches!(arg, GenericArgument::Type(_)))
    {
        return false;
    }
    let name = segment.ident.to_string();
    !matches!(
        name.as_str(),
        "Box"
            | "Rc"
            | "Arc"
            | "Cow"
            | "Cell"
            | "RefCell"
            | "Mutex"
            | "RwLock"
            | "Vec"
            | "LinkedList"
            | "BTreeSet"
            | "HashSet"
            | "Option"
            | "PhantomData"
            | "Result"
            | "HashMap"
            | "BTreeMap"
            | "DateTime"
    )
}

pub(crate) fn encoder_expr(ty: &TokenStream2, ctx: &Ctx<'_>) -> syn::Result<TokenStream2> {
    let parsed = parse_type(ty)?;
    if !contains_param(&parsed, ctx.params) {
        return Ok(quote_trait(ty, "::rs2elm::ElmEncode", "encoder_type"));
    }
    structural_codec(&parsed, ctx, ty.span(), Codec::Encode)
}

pub(crate) fn decoder_expr(ty: &TokenStream2, params: &[Param]) -> syn::Result<TokenStream2> {
    let parsed = parse_type(ty)?;
    if !contains_param(&parsed, params) {
        return Ok(quote_trait(ty, "::rs2elm::ElmDecode", "decoder_type"));
    }
    let ctx = Ctx {
        params,
        rewrite_parent: None,
        parent_ident: None,
    };
    structural_codec(&parsed, &ctx, ty.span(), Codec::Decode)
}

pub(crate) struct QueryEmit {
    /// Elm expression of type `List Url.Builder.QueryParameter`.
    pub(crate) list: TokenStream2,
    /// Elm expression of type `String -> List (String, String) -> Result String a`.
    pub(crate) parser: TokenStream2,
    /// Statements that register the query codecs this field calls. `module` is in scope.
    pub(crate) contribute: TokenStream2,
}

pub(crate) fn query_emit(
    ty: &TokenStream2,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
) -> syn::Result<QueryEmit> {
    let parsed = parse_type(ty)?;
    emit_query(&parsed, key, value, params, index, ty.span())
}

fn emit_query(
    ty: &Type,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    match ty {
        Type::Reference(reference) => emit_query(&reference.elem, key, value, params, index, span),
        Type::Paren(inner) => emit_query(&inner.elem, key, value, params, index, span),
        Type::Group(inner) => emit_query(&inner.elem, key, value, params, index, span),
        Type::Slice(slice) => emit_repeat(&slice.elem, key, value, params, index, span),
        Type::Array(array) => emit_repeat(&array.elem, key, value, params, index, span),
        Type::Path(path) => emit_query_path(path, key, value, params, index, span),
        _ => Err(no_query(ty, span)),
    }
}

fn emit_repeat(
    elem: &Type,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    let element_key = if index {
        format!("({key} ++ \"[\" ++ String.fromInt index ++ \"]\")")
    } else {
        key.to_string()
    };
    // `item` avoids shadowing the `value` bound by an inner `Just value` case.
    let element_name = if index { "item" } else { "value" };
    let inner = emit_query(elem, &element_key, element_name, params, false, span)?;
    let list = inner.list;
    let list = if index {
        quote::quote! {
            ::std::format!(
                "List.concat (List.indexedMap (\\index item -> {inner}) {value})",
                inner = #list,
                value = #value,
            )
        }
    } else {
        quote::quote! {
            ::std::format!(
                "List.concatMap (\\value -> {inner}) {value}",
                inner = #list,
                value = #value,
            )
        }
    };
    let (parser, contribute) = if index {
        if let Some(inner_ty) = option_elem(elem) {
            let leaf = emit_query(inner_ty, &element_key, element_name, params, false, span)?;
            let parser = wrap_parser("elmRsFormIndexedMaybe", leaf.parser);
            let contribute = {
                let first = inner.contribute;
                let second = leaf.contribute;
                quote::quote! {
                    #first
                    #second
                }
            };
            (parser, contribute)
        } else {
            (
                wrap_parser("elmRsFormIndexed", inner.parser),
                inner.contribute,
            )
        }
    } else {
        (
            wrap_parser("elmRsFormRepeated", inner.parser),
            inner.contribute,
        )
    };
    Ok(QueryEmit {
        list,
        parser,
        contribute,
    })
}

fn emit_option(
    elem: &Type,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    let inner = emit_query(elem, key, "value", params, index, span)?;
    let list = inner.list;
    let parser = inner.parser;
    Ok(QueryEmit {
        list: quote::quote! {
            ::std::format!(
                "(case {value} of\n        Nothing ->\n            []\n        Just value ->\n            {inner})",
                value = #value,
                inner = #list,
            )
        },
        parser: wrap_parser("elmRsFormOptional", parser),
        contribute: inner.contribute,
    })
}

/// `Option<T>` after references are peeled. Smart pointers stay in place.
fn option_elem(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(reference) => option_elem(&reference.elem),
        Type::Paren(inner) => option_elem(&inner.elem),
        Type::Group(inner) => option_elem(&inner.elem),
        Type::Path(path) => {
            let segment = path.path.segments.last()?;
            if segment.ident != "Option" {
                return None;
            }
            let args = angle_args(segment)?;
            only_type_arg(args, proc_macro2::Span::call_site()).ok()
        }
        _ => None,
    }
}

fn wrap_parser(helper: &str, inner: TokenStream2) -> TokenStream2 {
    let template = format!("({helper} {{inner}})");
    quote::quote! {
        {
            let inner = #inner;
            ::std::format!(#template, inner = inner)
        }
    }
}

fn emit_query_path(
    path: &TypePath,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    if let Some(var) = param_var(path, params) {
        if index {
            return Err(index_requires_list(span));
        }
        let ident = syn::Ident::new(
            params
                .iter()
                .find(|param| param.var == var)
                .map(|param| param.ident.as_str())
                .unwrap_or(var),
            span,
        );
        let encoder = format!("{var}Query");
        let form = format!("(elmRsFormText {var}Form)");
        return Ok(QueryEmit {
            list: quote::quote! {
                ::std::format!(
                    "[ Url.Builder.string {key} ({encoder} {value}) ]",
                    key = #key,
                    encoder = #encoder,
                    value = #value,
                )
            },
            parser: quote::quote! { ::std::string::String::from(#form) },
            contribute: quote::quote! {
                <#ident as ::rs2elm::ElmQueryField>::contribute_query_field(module);
            },
        });
    }
    let segment = path
        .path
        .segments
        .last()
        .ok_or_else(|| no_query_path(path, span))?;
    let name = segment.ident.to_string();
    if is_datetime(&name) {
        return leaf_emit(&path.to_token_stream(), key, value, index, span);
    }
    if matches!(
        name.as_str(),
        "HashMap" | "BTreeMap" | "Result" | "PhantomData"
    ) {
        return Err(no_query_path(path, span));
    }
    if let Some(args) = angle_args(segment) {
        if matches!(
            name.as_str(),
            "Box" | "Rc" | "Arc" | "Cow" | "Cell" | "RefCell" | "Mutex" | "RwLock"
        ) {
            return emit_query(only_type_arg(args, span)?, key, value, params, index, span);
        }
        if matches!(name.as_str(), "Vec" | "LinkedList" | "BTreeSet" | "HashSet") {
            return emit_repeat(only_type_arg(args, span)?, key, value, params, index, span);
        }
        if name == "Option" {
            return emit_option(only_type_arg(args, span)?, key, value, params, index, span);
        }
        return emit_applied(path, args, key, value, params, index, span);
    }
    leaf_emit(&path.to_token_stream(), key, value, index, span)
}

fn emit_applied(
    path: &TypePath,
    args: &syn::punctuated::Punctuated<GenericArgument, syn::Token![,]>,
    key: &str,
    value: &str,
    params: &[Param],
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    if index {
        return Err(index_requires_list(span));
    }
    let full = path.to_token_stream();
    let mut saw_param = false;
    let mut encoders = Vec::new();
    let mut forms = Vec::new();
    for arg in args {
        match arg {
            GenericArgument::Lifetime(_) => {}
            GenericArgument::Type(inner) => {
                if contains_param(inner, params) {
                    saw_param = true;
                }
                encoders.push(query_encoder(inner, params, span)?);
                forms.push(query_form(inner, params, span)?);
            }
            _ => {
                return Err(syn::Error::new(
                    span,
                    "cannot derive ElmQuery for this generic argument",
                ));
            }
        }
    }
    if !saw_param {
        return leaf_emit(&full, key, value, false, span);
    }
    Ok(QueryEmit {
        list: quote::quote! {
            {
                let parts = [ #(#encoders),* ];
                let args = ::std::format!("{} ", parts.join(" "));
                ::std::format!(
                    "elmRsQuery{elm} {args}{key} {value}",
                    elm = <#full as ::rs2elm::Elm>::elm_type(),
                    args = args,
                    key = #key,
                    value = #value,
                )
            }
        },
        parser: quote::quote! {
            {
                let parts = [ #(#forms),* ];
                let args = ::std::format!("{} ", parts.join(" "));
                ::std::format!(
                    "(elmRsDecode{elm} {args})",
                    elm = <#full as ::rs2elm::Elm>::elm_type(),
                    args = args,
                )
            }
        },
        contribute: quote::quote! {
            <#full as ::rs2elm::ElmQuery>::contribute_query(module);
        },
    })
}

fn query_form(ty: &Type, params: &[Param], span: proc_macro2::Span) -> syn::Result<TokenStream2> {
    if let Type::Path(path) = ty {
        if let Some(var) = param_var(path, params) {
            let form = format!("{var}Form");
            return Ok(quote::quote! { ::std::string::String::from(#form) });
        }
    }
    if contains_param(ty, params) {
        return Err(syn::Error::new(
            span,
            format!(
                "cannot derive ElmQuery for `{}` because this type argument contains a type parameter",
                type_string(ty)
            ),
        ));
    }
    let tokens = ty.to_token_stream();
    Ok(quote::quote! {
        ::std::string::String::from(<#tokens as ::rs2elm::ElmQueryField>::query_field_form_value())
    })
}

fn query_encoder(
    ty: &Type,
    params: &[Param],
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    if let Type::Path(path) = ty {
        if let Some(var) = param_var(path, params) {
            let encoder = format!("{var}Query");
            return Ok(quote::quote! { ::std::string::String::from(#encoder) });
        }
    }
    if contains_param(ty, params) {
        return Err(syn::Error::new(
            span,
            format!(
                "cannot derive ElmQuery for `{}` because this type argument contains a type parameter",
                type_string(ty)
            ),
        ));
    }
    let tokens = ty.to_token_stream();
    Ok(quote::quote! {
        ::std::string::String::from(<#tokens as ::rs2elm::ElmQueryField>::query_field_encoder_name())
    })
}

fn leaf_emit(
    ty: &TokenStream2,
    key: &str,
    value: &str,
    index: bool,
    span: proc_macro2::Span,
) -> syn::Result<QueryEmit> {
    if index {
        return Err(index_requires_list(span));
    }
    Ok(QueryEmit {
        list: quote::quote! {
            {
                use ::rs2elm::QueryLeaf as _;
                (&&&&::rs2elm::QueryDispatch::<#ty>(::std::marker::PhantomData)).query_list(#key, #value)
            }
        },
        parser: quote::quote! {
            {
                use ::rs2elm::QueryLeaf as _;
                (&&&&::rs2elm::QueryDispatch::<#ty>(::std::marker::PhantomData)).form_parser()
            }
        },
        contribute: quote::quote! {
            {
                use ::rs2elm::QueryLeaf as _;
                (&&&&::rs2elm::QueryDispatch::<#ty>(::std::marker::PhantomData)).query_contribute(module);
            }
        },
    })
}

fn index_requires_list(span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(span, "`#[rs2elm(index)]` requires a list")
}

fn no_query(ty: &Type, span: proc_macro2::Span) -> syn::Error {
    syn::Error::new(
        span,
        format!(
            "cannot derive ElmQuery for `{}` because this type has no query codec",
            type_string(ty)
        ),
    )
}

fn no_query_path(path: &TypePath, span: proc_macro2::Span) -> syn::Error {
    no_query(&Type::Path(path.clone()), span)
}

fn parse_type(ty: &TokenStream2) -> syn::Result<Type> {
    syn::parse2(ty.clone()).map_err(|err| syn::Error::new(ty.span(), err.to_string()))
}

fn quote_trait(ty: &TokenStream2, trait_path: &str, method: &str) -> TokenStream2 {
    let trait_path: syn::Path = syn::parse_str(trait_path).expect("trait path");
    let method = syn::Ident::new(method, proc_macro2::Span::call_site());
    quote::quote! { <#ty as #trait_path>::#method() }
}

#[derive(Clone, Copy)]
enum Codec {
    Encode,
    Decode,
}

fn type_string(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

/// `chrono::DateTime<Tz>` is always an Elm `String`. Its parameter is a time zone, not a value.
fn is_datetime(name: &str) -> bool {
    name == "DateTime"
}

fn structural_elm(
    ty: &Type,
    params: &[Param],
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    match ty {
        Type::Path(path) => {
            if let Some(var) = param_var(path, params) {
                return Ok(literal(var));
            }
            let segment = path
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new(span, "cannot derive Elm for this type"))?;
            let name = segment.ident.to_string();
            if is_datetime(&name) {
                let tokens = ty.to_token_stream();
                return Ok(quote_trait(&tokens, "::rs2elm::Elm", "elm_type"));
            }
            if let Some(args) = angle_args(segment) {
                if matches!(
                    name.as_str(),
                    "Box" | "Rc" | "Arc" | "Cow" | "Cell" | "RefCell" | "Mutex" | "RwLock"
                ) {
                    let inner = only_type_arg(args, span)?;
                    return structural_elm(inner, params, span);
                }
                if matches!(name.as_str(), "Vec" | "LinkedList" | "BTreeSet" | "HashSet") {
                    let inner = structural_elm(only_type_arg(args, span)?, params, span)?;
                    return Ok(wrap("List ({})", inner));
                }
                if name == "Option" {
                    let inner = structural_elm(only_type_arg(args, span)?, params, span)?;
                    return Ok(wrap("Maybe ({})", inner));
                }
                if name == "PhantomData" {
                    return Ok(literal("()"));
                }
                if name == "Result" {
                    let (ok, err) = two_type_args(args, span)?;
                    let err = structural_elm(err, params, span)?;
                    let ok = structural_elm(ok, params, span)?;
                    return Ok(wrap2("Result ({}) ({})", err, ok));
                }
                if matches!(name.as_str(), "HashMap" | "BTreeMap") {
                    let (key, value) = two_type_args(args, span)?;
                    require_string_key(key, span)?;
                    let value = structural_elm(value, params, span)?;
                    return Ok(wrap("Dict String ({})", value));
                }
                if args
                    .iter()
                    .any(|arg| matches!(arg, GenericArgument::Type(_)))
                {
                    return user_applied(path, params, span, Applied::Elm);
                }
            }
            let tokens = ty.to_token_stream();
            Ok(quote_trait(&tokens, "::rs2elm::Elm", "elm_type"))
        }
        Type::Reference(reference) => structural_elm(&reference.elem, params, span),
        Type::Slice(slice) => {
            let inner = structural_elm(&slice.elem, params, span)?;
            Ok(wrap("List ({})", inner))
        }
        Type::Array(array) => {
            let inner = structural_elm(&array.elem, params, span)?;
            Ok(wrap("List ({})", inner))
        }
        Type::Tuple(tuple) => tuple_elm(&tuple.elems, params, span),
        Type::Paren(inner) => structural_elm(&inner.elem, params, span),
        Type::Group(inner) => structural_elm(&inner.elem, params, span),
        _ => {
            if contains_param(ty, params) {
                Err(syn::Error::new(
                    span,
                    format!(
                        "cannot derive Elm for `{}` because it contains a type parameter",
                        type_string(ty)
                    ),
                ))
            } else {
                let tokens = ty.to_token_stream();
                Ok(quote_trait(&tokens, "::rs2elm::Elm", "elm_type"))
            }
        }
    }
}

fn structural_codec(
    ty: &Type,
    ctx: &Ctx<'_>,
    span: proc_macro2::Span,
    codec: Codec,
) -> syn::Result<TokenStream2> {
    if !contains_param(ty, ctx.params) {
        let tokens = ty.to_token_stream();
        return Ok(match codec {
            Codec::Encode => quote_trait(&tokens, "::rs2elm::ElmEncode", "encoder_type"),
            Codec::Decode => quote_trait(&tokens, "::rs2elm::ElmDecode", "decoder_type"),
        });
    }
    match ty {
        Type::Path(path) => {
            if let Some(var) = param_var(path, ctx.params) {
                let name = match codec {
                    Codec::Encode => format!("{var}Encoder"),
                    Codec::Decode => format!("{var}Decoder"),
                };
                return Ok(literal(&name));
            }
            let segment = path
                .path
                .segments
                .last()
                .ok_or_else(|| syn::Error::new(span, "cannot derive a codec for this type"))?;
            let name = segment.ident.to_string();
            if is_datetime(&name) {
                let tokens = ty.to_token_stream();
                return Ok(quote_trait(&tokens, trait_for(codec), method_for(codec)));
            }
            let args = type_args(segment)?;
            if matches!(
                name.as_str(),
                "Box" | "Rc" | "Arc" | "Cow" | "Cell" | "RefCell" | "Mutex" | "RwLock"
            ) {
                return structural_codec(only_type_arg(&args, span)?, ctx, span, codec);
            }
            if matches!(name.as_str(), "Vec" | "LinkedList" | "BTreeSet" | "HashSet") {
                let inner = structural_codec(only_type_arg(&args, span)?, ctx, span, codec)?;
                let template = match codec {
                    Codec::Encode => "Json.Encode.list ({})",
                    Codec::Decode => "Json.Decode.list ({})",
                };
                return Ok(wrap(template, inner));
            }
            if name == "Option" {
                let inner = structural_codec(only_type_arg(&args, span)?, ctx, span, codec)?;
                let template = match codec {
                    Codec::Encode => "Maybe.withDefault Json.Encode.null << Maybe.map ({})",
                    Codec::Decode => "Json.Decode.nullable ({})",
                };
                return Ok(wrap(template, inner));
            }
            if name == "PhantomData" {
                return Ok(match codec {
                    Codec::Encode => literal(r"(\_ -> Json.Encode.null)"),
                    Codec::Decode => literal("Json.Decode.null ()"),
                });
            }
            if name == "Result" {
                let (ok, err) = two_type_args(&args, span)?;
                let err = structural_codec(err, ctx, span, codec)?;
                let ok = structural_codec(ok, ctx, span, codec)?;
                let template = match codec {
                    Codec::Encode => "resultEncoder ({}) ({})",
                    Codec::Decode => "resultDecoder ({}) ({})",
                };
                return Ok(wrap2(template, err, ok));
            }
            if matches!(name.as_str(), "HashMap" | "BTreeMap") {
                let (key, value) = two_type_args(&args, span)?;
                require_string_key(key, span)?;
                let value = structural_codec(value, ctx, span, codec)?;
                let template = match codec {
                    Codec::Encode => "Json.Encode.dict identity ({})",
                    Codec::Decode => "Json.Decode.dict ({})",
                };
                return Ok(wrap(template, value));
            }
            user_applied_codec(path, ctx, span, codec)
        }
        Type::Reference(reference) => structural_codec(&reference.elem, ctx, span, codec),
        Type::Slice(slice) => {
            let inner = structural_codec(&slice.elem, ctx, span, codec)?;
            let template = match codec {
                Codec::Encode => "Json.Encode.list ({})",
                Codec::Decode => "Json.Decode.list ({})",
            };
            Ok(wrap(template, inner))
        }
        Type::Array(array) => {
            let inner = structural_codec(&array.elem, ctx, span, codec)?;
            match codec {
                Codec::Encode => Ok(wrap("Json.Encode.list ({})", inner)),
                Codec::Decode => {
                    // Match `[T; N]::decoder_type`: length-check the slice decoder, which is `Json.Decode.list`.
                    let len = array.len.to_token_stream().to_string().replace(' ', "");
                    let list = wrap("Json.Decode.list ({})", inner);
                    Ok(quote::quote! {
                        ::std::format!(
                            "Json.Decode.andThen (\\list -> if List.length list == {len} then Json.Decode.succeed list else Json.Decode.fail \"expected a list of length {len}\") ({inner})",
                            len = #len,
                            inner = #list,
                        )
                    })
                }
            }
        }
        Type::Paren(inner) => structural_codec(&inner.elem, ctx, span, codec),
        Type::Group(inner) => structural_codec(&inner.elem, ctx, span, codec),
        _ => Err(syn::Error::new(
            span,
            format!(
                "cannot derive an Elm codec for `{}` because it contains a type parameter",
                type_string(ty)
            ),
        )),
    }
}

enum Applied {
    Elm,
}

fn user_applied(
    path: &TypePath,
    params: &[Param],
    span: proc_macro2::Span,
    _kind: Applied,
) -> syn::Result<TokenStream2> {
    let segment = path.path.segments.last().unwrap();
    let args = type_args(segment)?;
    let full = path.to_token_stream();
    let mut arg_exprs = Vec::new();
    for arg in args {
        match arg {
            GenericArgument::Type(inner) => arg_exprs.push(structural_elm(inner, params, span)?),
            GenericArgument::Lifetime(_) => {}
            _ => {
                return Err(syn::Error::new(
                    span,
                    "cannot derive Elm for this generic argument",
                ));
            }
        }
    }
    Ok(quote::quote! {
        {
            let mut out = <#full as ::rs2elm::Elm>::elm_type();
            let args: &[::std::string::String] = &[#(#arg_exprs),*];
            for arg in args {
                out.push(' ');
                if arg.contains(' ') || arg.contains('(') {
                    out.push('(');
                    out.push_str(arg);
                    out.push(')');
                } else {
                    out.push_str(arg);
                }
            }
            out
        }
    })
}

fn user_applied_codec(
    path: &TypePath,
    ctx: &Ctx<'_>,
    span: proc_macro2::Span,
    codec: Codec,
) -> syn::Result<TokenStream2> {
    let segment = path.path.segments.last().unwrap();
    if let (Some(parent), Codec::Encode) = (ctx.parent_ident, codec) {
        if segment.ident == parent && args_are_params(segment, ctx.params)? {
            if let Some(name) = ctx.rewrite_parent {
                return Ok(literal(name));
            }
        }
    }
    let full = path.to_token_stream();
    let args = type_args(segment)?;
    let mut arg_exprs = Vec::new();
    for arg in args {
        match arg {
            GenericArgument::Type(inner) => {
                arg_exprs.push(structural_codec(inner, ctx, span, codec)?)
            }
            GenericArgument::Lifetime(_) => {}
            _ => {
                return Err(syn::Error::new(
                    span,
                    "cannot derive an Elm codec for this generic argument",
                ));
            }
        }
    }
    let method = match codec {
        Codec::Encode => quote::quote! { encoder_name },
        Codec::Decode => quote::quote! { decoder_name },
    };
    let trait_path = match codec {
        Codec::Encode => quote::quote! { ::rs2elm::ElmEncode },
        Codec::Decode => quote::quote! { ::rs2elm::ElmDecode },
    };
    Ok(quote::quote! {
        {
            let name = <#full as #trait_path>::#method();
            let args: &[::std::string::String] = &[#(#arg_exprs),*];
            let mut out = ::std::string::String::from("(");
            out.push_str(&name);
            for arg in args {
                out.push(' ');
                out.push('(');
                out.push_str(arg);
                out.push(')');
            }
            out.push(')');
            out
        }
    })
}

fn args_are_params(segment: &syn::PathSegment, params: &[Param]) -> syn::Result<bool> {
    let args = type_args(segment)?;
    let types: Vec<&Type> = args
        .iter()
        .filter_map(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect();
    if types.len() != params.len() {
        return Ok(false);
    }
    Ok(types.iter().zip(params).all(|(ty, param)| {
        matches!(ty, Type::Path(path) if path_is_param(path, std::slice::from_ref(param)))
    }))
}

fn trait_for(codec: Codec) -> &'static str {
    match codec {
        Codec::Encode => "::rs2elm::ElmEncode",
        Codec::Decode => "::rs2elm::ElmDecode",
    }
}

fn method_for(codec: Codec) -> &'static str {
    match codec {
        Codec::Encode => "encoder_type",
        Codec::Decode => "decoder_type",
    }
}

fn angle_args(
    segment: &syn::PathSegment,
) -> Option<&syn::punctuated::Punctuated<GenericArgument, syn::Token![,]>> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => Some(&args.args),
        _ => None,
    }
}

fn type_args(
    segment: &syn::PathSegment,
) -> syn::Result<&syn::punctuated::Punctuated<GenericArgument, syn::Token![,]>> {
    match angle_args(segment) {
        Some(args) => Ok(args),
        None => Err(syn::Error::new(
            segment.ident.span(),
            format!(
                "cannot derive Elm for `{}` because it contains a type parameter",
                segment.ident
            ),
        )),
    }
}

fn only_type_arg<'a>(
    args: &'a syn::punctuated::Punctuated<GenericArgument, syn::Token![,]>,
    span: proc_macro2::Span,
) -> syn::Result<&'a Type> {
    let types: Vec<&Type> = args
        .iter()
        .filter_map(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect();
    if types.len() != 1 {
        return Err(syn::Error::new(span, "expected one type argument"));
    }
    Ok(types[0])
}

fn two_type_args<'a>(
    args: &'a syn::punctuated::Punctuated<GenericArgument, syn::Token![,]>,
    span: proc_macro2::Span,
) -> syn::Result<(&'a Type, &'a Type)> {
    let types: Vec<&Type> = args
        .iter()
        .filter_map(|arg| match arg {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect();
    if types.len() != 2 {
        return Err(syn::Error::new(span, "expected two type arguments"));
    }
    Ok((types[0], types[1]))
}

fn require_string_key(key: &Type, span: proc_macro2::Span) -> syn::Result<()> {
    let Type::Path(path) = key else {
        return Err(syn::Error::new(
            span,
            "map keys that contain a type parameter are not supported; the key must be String",
        ));
    };
    let name = path
        .path
        .segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default();
    if name == "String" || name == "str" {
        Ok(())
    } else {
        Err(syn::Error::new(
            span,
            format!(
                "map keys that contain a type parameter are not supported; the key must be String, found `{name}`"
            ),
        ))
    }
}

fn tuple_elm(
    elems: &syn::punctuated::Punctuated<Type, syn::Token![,]>,
    params: &[Param],
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    let parts: Vec<TokenStream2> = elems
        .iter()
        .map(|elem| structural_elm(elem, params, span))
        .collect::<Result<Vec<_>, _>>()?;
    match parts.len() {
        1 => Ok(parts.into_iter().next().unwrap()),
        2 => Ok(wrap2("( {}, {} )", parts[0].clone(), parts[1].clone())),
        3 => {
            let first = &parts[0];
            let second = &parts[1];
            let third = &parts[2];
            Ok(quote::quote! {
                ::std::format!("( {}, {}, {} )", #first, #second, #third)
            })
        }
        _ => Err(syn::Error::new(
            span,
            "tuples with a type parameter support at most 3 elements",
        )),
    }
}

fn literal(value: &str) -> TokenStream2 {
    quote::quote! { ::std::string::String::from(#value) }
}

fn wrap(template: &str, inner: TokenStream2) -> TokenStream2 {
    quote::quote! { ::std::format!(#template, #inner) }
}

fn wrap2(template: &str, first: TokenStream2, second: TokenStream2) -> TokenStream2 {
    quote::quote! { ::std::format!(#template, #first, #second) }
}

trait SpanTokens {
    fn span(&self) -> proc_macro2::Span;
}

impl SpanTokens for TokenStream2 {
    fn span(&self) -> proc_macro2::Span {
        use proc_macro2::Span;
        self.clone()
            .into_iter()
            .next()
            .map(|token| token.span())
            .unwrap_or_else(Span::call_site)
    }
}
