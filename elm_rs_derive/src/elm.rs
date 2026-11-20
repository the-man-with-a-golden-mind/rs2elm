//! Derive macro for Elm.

use super::{
    EnumVariant, EnumVariantKind, Intermediate, StructField, TypeInfo,
    polymorph::{self, Forms},
};
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
        container_attributes: _,
    }: Intermediate,
) -> syn::Result<TokenStream2> {
    let forms = Forms::from_generics(&elm_type, &generics);
    let contribute_fields = super::contribute_calls(
        &super::dependency_types(&type_info),
        quote! { ::rs2elm::Elm },
        "contribute",
    );
    let constructors: Vec<String> = match &type_info {
        TypeInfo::Unit | TypeInfo::Newtype(_) | TypeInfo::Tuple(_) => vec![elm_type.clone()],
        TypeInfo::Struct(fields) if fields.iter().any(|field| field.lazy) => {
            vec![elm_type.clone()]
        }
        TypeInfo::Struct(_) => Vec::new(),
        TypeInfo::Enum { variants, .. } => variants
            .iter()
            .filter(|variant| {
                #[cfg(feature = "serde")]
                {
                    !variant.serde_attributes.skip
                }
                #[cfg(not(feature = "serde"))]
                {
                    let _ = variant;
                    true
                }
            })
            .map(|variant| variant.name_elm().into_owned())
            .collect(),
    };

    let type_definition = match type_info {
        TypeInfo::Unit => unit(&elm_type, &forms),
        TypeInfo::Newtype(field) => newtype(&elm_type, &forms, &field.ty)?,
        TypeInfo::Tuple(fields) => tuple(&elm_type, &forms, &fields)?,
        TypeInfo::Struct(fields) => struct_type(&elm_type, &forms, fields)?,
        TypeInfo::Enum { variants, .. } => enum_type(&elm_type, &forms, variants)?,
    };

    for p in generics.type_params_mut() {
        p.bounds.push(syn::parse_str("::rs2elm::Elm").unwrap());
    }

    let token_stream = quote! {
        impl #generics ::rs2elm::Elm for #ident #generics_without_bounds {
            fn elm_type() -> ::std::string::String {
                ::std::convert::From::from(#elm_type)
            }

            fn elm_definition() -> ::std::option::Option<::std::string::String> {
                ::std::option::Option::Some(#type_definition)
            }

            fn elm_constructors() -> &'static [&'static str] {
                &[#(#constructors),*]
            }

            fn contribute(module: &mut ::rs2elm::export::Module) {
                if !module.mark(&<Self as ::rs2elm::Elm>::elm_type()) {
                    return;
                }
                #contribute_fields
                for constructor in <Self as ::rs2elm::Elm>::elm_constructors() {
                    module.note_constructor(constructor, &<Self as ::rs2elm::Elm>::elm_type());
                }
                if let ::std::option::Option::Some(definition) = <Self as ::rs2elm::Elm>::elm_definition() {
                    module.push_definition(&definition);
                }
            }
        }
    };
    Ok(token_stream)
}

fn unit(elm_type: &str, forms: &Forms) -> TokenStream2 {
    let applied = &forms.applied;
    quote! {::std::format!("\
type {applied}
    = {elm_type}
",
        applied = #applied,
        elm_type = #elm_type,
    )}
}

fn newtype(elm_type: &str, forms: &Forms, ty: &TokenStream2) -> syn::Result<TokenStream2> {
    let applied = &forms.applied;
    let inner = polymorph::elm_type_expr(ty, &forms.params)?;
    Ok(quote! {::std::format!("\
type {applied}
    = {elm_type} ({inner_type})
",
        applied = #applied,
        elm_type = #elm_type,
        inner_type = #inner,
    )})
}

fn tuple(
    elm_type: &str,
    forms: &Forms,
    fields: &[super::PositionalField],
) -> syn::Result<TokenStream2> {
    let applied = &forms.applied;
    let mut types = Vec::new();
    for field in fields {
        let elm = polymorph::elm_type_expr(&field.ty, &forms.params)?;
        types.push(quote! { ::std::format!("({})", #elm) });
    }
    Ok(quote! {::std::format!("\
type {applied}
    = {elm_type} {types}
",
        applied = #applied,
        elm_type = #elm_type,
        types = (&[#(#types),*]).join(" "),
    )})
}

fn struct_type(
    elm_type: &str,
    forms: &Forms,
    fields: Vec<StructField>,
) -> syn::Result<TokenStream2> {
    let applied = &forms.applied;
    let recursive = fields.iter().any(|field| field.lazy);
    let mut field_lines = Vec::new();
    for field in &fields {
        let id = field.name_elm();
        let elm = polymorph::elm_type_expr(&field.ty, &forms.params)?;
        field_lines.push(quote! { ::std::format!("{} : {}", #id, #elm) });
    }
    let field_lines = quote! { (&[#(#field_lines),*]) };
    if recursive {
        Ok(quote! {::std::format!("\
type {applied}
    = {elm_type}
        {{ {fields}
        }}
",
            applied = #applied,
            elm_type = #elm_type,
            fields = #field_lines.join("\n        , "),
        )})
    } else {
        Ok(quote! {::std::format!("\
type alias {applied} =
    {{ {fields}
    }}
",
            applied = #applied,
            fields = #field_lines.join("\n    , "),
        )})
    }
}

fn enum_type(
    _elm_type: &str,
    forms: &Forms,
    enum_variants: Vec<EnumVariant>,
) -> syn::Result<TokenStream2> {
    let applied = &forms.applied;
    let mut enum_fields: Vec<TokenStream2> = vec![];
    for enum_variant in enum_variants {
        #[cfg(feature = "serde")]
        if enum_variant.serde_attributes.skip {
            continue;
        }
        let variant_elm_name = enum_variant.name_elm();
        match &enum_variant.variant {
            EnumVariantKind::Unit => {
                enum_fields.push(quote! { #variant_elm_name });
            }
            EnumVariantKind::Newtype(field) => {
                let inner = polymorph::elm_type_expr(&field.ty, &forms.params)?;
                enum_fields.push(quote! {
                    ::std::format!("{} ({})", #variant_elm_name, #inner)
                });
            }
            EnumVariantKind::Tuple(fields) => {
                let mut types = Vec::new();
                for field in fields {
                    let inner = polymorph::elm_type_expr(&field.ty, &forms.params)?;
                    types.push(quote! { ::std::format!("({})", #inner) });
                }
                enum_fields.push(quote! {
                    ::std::format!("{name} {types}",
                        name = #variant_elm_name,
                        types = (&[#(#types),*]).join(" "))
                });
            }
            EnumVariantKind::Struct(fields) => {
                let mut parts = Vec::new();
                for field in fields {
                    let id = field.name_elm();
                    let inner = polymorph::elm_type_expr(&field.ty, &forms.params)?;
                    parts.push(quote! { ::std::format!("{} : {}", #id, #inner) });
                }
                enum_fields.push(quote! {
                    ::std::format!("{name} {{ {fields} }}",
                        name = #variant_elm_name,
                        fields = (&[#(#parts),*]).join(", "))
                });
            }
        }
    }
    Ok(quote! {::std::format!("\
type {applied}
    = {enum_fields}
",
        applied = #applied,
        enum_fields =
            (
                &[
                    #(::std::format!("{}", #enum_fields)),*
                ] as &[::std::string::String]
            ).join("\n    | "),
    )})
}
