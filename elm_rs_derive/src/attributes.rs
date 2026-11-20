//! Parsing macro attributes.

use syn::{Attribute, Token};

#[derive(Default, Clone)]
pub(crate) struct ElmAttr {
    pub(crate) rename: Option<String>,
    pub(crate) lazy: bool,
    /// Number elements of a list field: `name[0]`, `name[1]`, ….
    pub(crate) index: bool,
}

impl ElmAttr {
    pub(crate) fn parse(attr: &Attribute, allow_lazy: bool) -> syn::Result<Self> {
        let mut elm = Self::default();
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") && meta.input.parse::<Token![=]>().is_ok() {
                elm.rename = Some(meta.input.parse::<syn::LitStr>()?.value());
                return Ok(());
            }
            if meta.path.is_ident("lazy") {
                if !allow_lazy {
                    return Err(meta.error("`lazy` is only valid on a field"));
                }
                elm.lazy = true;
                return Ok(());
            }
            if meta.path.is_ident("index") {
                if !allow_lazy {
                    return Err(meta.error("`index` is only valid on a field"));
                }
                elm.index = true;
                return Ok(());
            }
            Err(meta.error("unknown rs2elm attribute"))
        })?;
        Ok(elm)
    }
}

#[derive(Default)]
pub(crate) struct ContainerAttributes {
    #[cfg(feature = "serde")]
    pub(crate) serde: serde::ContainerAttributes,
    pub(crate) elm: ElmAttr,
}

impl ContainerAttributes {
    pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut attributes = Self::default();
        for attr in attrs {
            #[cfg(feature = "serde")]
            if attr.path().is_ident("serde") {
                attributes.serde.parse(attr)?;
            }
            if attr.path().is_ident("rs2elm") {
                attributes.elm = ElmAttr::parse(attr, false)?;
            }
        }
        Ok(attributes)
    }
}

#[derive(Default)]
pub(crate) struct VariantAttributes {
    #[cfg(feature = "serde")]
    pub(crate) serde: serde::VariantAttributes,
    pub(crate) elm: ElmAttr,
}

impl VariantAttributes {
    pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut attributes = Self::default();

        for attr in attrs {
            #[cfg(feature = "serde")]
            if attr.path().is_ident("serde") {
                attributes.serde.parse(attr)?;
            }
            if attr.path().is_ident("rs2elm") {
                attributes.elm = ElmAttr::parse(attr, false)?;
            }
        }

        Ok(attributes)
    }
}

#[derive(Default)]
pub(crate) struct FieldAttributes {
    #[cfg(feature = "serde")]
    pub(crate) serde: serde::FieldAttributes,
    pub(crate) elm: ElmAttr,
}

impl FieldAttributes {
    pub(crate) fn parse(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut attributes = Self::default();

        for attr in attrs {
            #[cfg(feature = "serde")]
            if attr.path().is_ident("serde") {
                attributes.serde.parse(attr)?;
            }
            if attr.path().is_ident("rs2elm") {
                attributes.elm = ElmAttr::parse(attr, true)?;
            }
        }

        Ok(attributes)
    }
}

#[cfg(feature = "serde")]
pub(crate) mod serde {
    use heck::{
        ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutyKebabCase, ToShoutySnakeCase,
        ToSnakeCase,
    };
    use proc_macro2::Ident;
    use syn::{Attribute, LitStr, Token, token};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum SkipIf {
        OptionIsNone,
        VecIsEmpty,
        StringIsEmpty,
    }

    fn rename_rule(lit: LitStr) -> syn::Result<RenameAll> {
        RenameAll::from(&lit.value()).ok_or_else(|| {
            syn::Error::new(
                lit.span(),
                format!("unsupported serde rename rule `{}`", lit.value()),
            )
        })
    }

    fn skip_if_path(lit: LitStr) -> syn::Result<SkipIf> {
        match lit.value().as_str() {
            "Option::is_none" => Ok(SkipIf::OptionIsNone),
            "Vec::is_empty" => Ok(SkipIf::VecIsEmpty),
            "String::is_empty" => Ok(SkipIf::StringIsEmpty),
            other => Err(syn::Error::new(
                lit.span(),
                format!("unsupported skip_serializing_if path `{other}`"),
            )),
        }
    }

    fn parse_split_string(
        meta: &syn::meta::ParseNestedMeta<'_>,
        plain: &mut Option<String>,
        deserialize: &mut Option<String>,
        serialize: &mut Option<String>,
    ) -> syn::Result<()> {
        if meta.input.peek(token::Paren) {
            meta.parse_nested_meta(|nested| {
                if nested.input.parse::<Token![=]>().is_err() {
                    return Err(nested.error("expected `=`"));
                }
                let content = nested.input.parse::<LitStr>()?;
                if nested.path.is_ident("deserialize") {
                    *deserialize = Some(content.value());
                    return Ok(());
                }
                if nested.path.is_ident("serialize") {
                    *serialize = Some(content.value());
                    return Ok(());
                }
                Err(nested.error("unsupported serde rename option"))
            })
        } else if meta.input.parse::<Token![=]>().is_ok() {
            *plain = Some(meta.input.parse::<LitStr>()?.value());
            Ok(())
        } else {
            Err(meta.error("expected `=` or `(...)`"))
        }
    }

    fn parse_split_rule(
        meta: &syn::meta::ParseNestedMeta<'_>,
        plain: &mut Option<RenameAll>,
        deserialize: &mut Option<RenameAll>,
        serialize: &mut Option<RenameAll>,
    ) -> syn::Result<()> {
        if meta.input.peek(token::Paren) {
            meta.parse_nested_meta(|nested| {
                if nested.input.parse::<Token![=]>().is_err() {
                    return Err(nested.error("expected `=`"));
                }
                let rule = rename_rule(nested.input.parse::<LitStr>()?)?;
                if nested.path.is_ident("deserialize") {
                    *deserialize = Some(rule);
                    return Ok(());
                }
                if nested.path.is_ident("serialize") {
                    *serialize = Some(rule);
                    return Ok(());
                }
                Err(nested.error("unsupported serde rename option"))
            })
        } else if meta.input.parse::<Token![=]>().is_ok() {
            *plain = Some(rename_rule(meta.input.parse::<LitStr>()?)?);
            Ok(())
        } else {
            Err(meta.error("expected `=` or `(...)`"))
        }
    }

    fn expect_lit(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<String> {
        if meta.input.parse::<Token![=]>().is_err() {
            return Err(meta.error("expected `=`"));
        }
        Ok(meta.input.parse::<LitStr>()?.value())
    }

    fn parse_default_flag(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<bool> {
        if meta.input.peek(Token![=]) {
            let _ = meta.input.parse::<Token![=]>()?;
            let content = meta.input.parse::<LitStr>()?;
            return Err(syn::Error::new(
                content.span(),
                "the Elm side has no Rust `Default` value",
            ));
        }
        Ok(true)
    }

    #[derive(Clone, Copy)]
    pub(crate) enum RenameAll {
        Lowercase,
        Uppercase,
        PascalCase,
        CamelCase,
        SnakeCase,
        ScreamingSnakeCase,
        KebabCase,
        ScreamingKebabCase,
    }

    impl RenameAll {
        pub(crate) fn from(s: &str) -> Option<Self> {
            match s {
                "lowercase" => Some(RenameAll::Lowercase),
                "UPPERCASE" => Some(RenameAll::Uppercase),
                "PascalCase" => Some(RenameAll::PascalCase),
                "camelCase" => Some(RenameAll::CamelCase),
                "snake_case" => Some(RenameAll::SnakeCase),
                "SCREAMING_SNAKE_CASE" => Some(RenameAll::ScreamingSnakeCase),
                "kebab-case" => Some(RenameAll::KebabCase),
                "SCREAMING-KEBAB-CASE" => Some(RenameAll::ScreamingKebabCase),
                _ => None,
            }
        }

        pub(crate) fn rename_ident(self, ident: &Ident) -> String {
            match self {
                RenameAll::Lowercase => ident.to_string().to_lowercase(),
                RenameAll::Uppercase => ident.to_string().to_uppercase(),
                RenameAll::PascalCase => ident.to_string().to_pascal_case(),
                RenameAll::CamelCase => ident.to_string().to_lower_camel_case(),
                RenameAll::SnakeCase => ident.to_string().to_snake_case(),
                RenameAll::ScreamingSnakeCase => ident.to_string().to_shouty_snake_case(),
                RenameAll::KebabCase => ident.to_string().to_kebab_case(),
                RenameAll::ScreamingKebabCase => ident.to_string().to_shouty_kebab_case(),
            }
        }
    }

    #[derive(Default)]
    pub(crate) struct ContainerAttributes {
        pub(crate) rename: Option<String>,
        pub(crate) rename_deserialize: Option<String>,
        pub(crate) rename_serialize: Option<String>,
        pub(crate) rename_all: Option<RenameAll>,
        pub(crate) rename_all_deserialize: Option<RenameAll>,
        pub(crate) rename_all_serialize: Option<RenameAll>,
        pub(crate) rename_all_fields: Option<RenameAll>,
        pub(crate) rename_all_fields_deserialize: Option<RenameAll>,
        pub(crate) rename_all_fields_serialize: Option<RenameAll>,
        pub(crate) enum_representation: EnumRepresentation,
        pub(crate) transparent: bool,
        pub(crate) deny_unknown_fields: bool,
        pub(crate) default: bool,
    }

    impl ContainerAttributes {
        pub(crate) fn parse(&mut self, attr: &Attribute) -> syn::Result<()> {
            let mut tag_attr = None;
            let mut content_attr = None;

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    return parse_split_string(
                        &meta,
                        &mut self.rename,
                        &mut self.rename_deserialize,
                        &mut self.rename_serialize,
                    );
                }
                if meta.path.is_ident("rename_all") {
                    return parse_split_rule(
                        &meta,
                        &mut self.rename_all,
                        &mut self.rename_all_deserialize,
                        &mut self.rename_all_serialize,
                    );
                }
                if meta.path.is_ident("rename_all_fields") {
                    return parse_split_rule(
                        &meta,
                        &mut self.rename_all_fields,
                        &mut self.rename_all_fields_deserialize,
                        &mut self.rename_all_fields_serialize,
                    );
                }
                if meta.path.is_ident("tag") {
                    tag_attr = Some(expect_lit(&meta)?);
                    return Ok(());
                }
                if meta.path.is_ident("content") {
                    content_attr = Some(expect_lit(&meta)?);
                    return Ok(());
                }
                if meta.path.is_ident("untagged") {
                    self.enum_representation = EnumRepresentation::Untagged;
                    return Ok(());
                }
                if meta.path.is_ident("transparent") {
                    self.transparent = true;
                    return Ok(());
                }
                if meta.path.is_ident("deny_unknown_fields") {
                    self.deny_unknown_fields = true;
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    self.default = parse_default_flag(&meta)?;
                    return Ok(());
                }
                Err(meta.error("unsupported serde attribute"))
            })?;

            if let Some(tag) = tag_attr {
                if let Some(content) = content_attr {
                    self.enum_representation = EnumRepresentation::Adjacent { tag, content };
                } else {
                    self.enum_representation = EnumRepresentation::Internal { tag };
                }
            }

            Ok(())
        }
    }

    #[derive(Default)]
    pub(crate) struct VariantAttributes {
        pub(crate) rename: Option<String>,
        pub(crate) rename_deserialize: Option<String>,
        pub(crate) rename_serialize: Option<String>,
        pub(crate) rename_all: Option<RenameAll>,
        pub(crate) rename_all_deserialize: Option<RenameAll>,
        pub(crate) rename_all_serialize: Option<RenameAll>,
        pub(crate) aliases: Vec<String>,
        pub(crate) skip: bool,
        pub(crate) other: bool,
    }

    impl VariantAttributes {
        pub(crate) fn parse(&mut self, attr: &Attribute) -> syn::Result<()> {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    return parse_split_string(
                        &meta,
                        &mut self.rename,
                        &mut self.rename_deserialize,
                        &mut self.rename_serialize,
                    );
                }
                if meta.path.is_ident("alias") {
                    self.aliases.push(expect_lit(&meta)?);
                    return Ok(());
                }
                if meta.path.is_ident("rename_all") {
                    return parse_split_rule(
                        &meta,
                        &mut self.rename_all,
                        &mut self.rename_all_deserialize,
                        &mut self.rename_all_serialize,
                    );
                }
                if meta.path.is_ident("skip") {
                    self.skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("other") {
                    self.other = true;
                    return Ok(());
                }
                Err(meta.error("unsupported serde attribute"))
            })?;

            Ok(())
        }
    }

    #[derive(Default)]
    pub(crate) struct FieldAttributes {
        pub(crate) rename: Option<String>,
        pub(crate) rename_deserialize: Option<String>,
        pub(crate) rename_serialize: Option<String>,
        pub(crate) aliases: Vec<String>,
        pub(crate) flatten: bool,
        pub(crate) skip: bool,
        pub(crate) skip_serializing: bool,
        pub(crate) skip_deserializing: bool,
        pub(crate) skip_serializing_if: Option<SkipIf>,
        pub(crate) default: bool,
    }

    impl FieldAttributes {
        pub(crate) fn parse(&mut self, attr: &Attribute) -> syn::Result<()> {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    return parse_split_string(
                        &meta,
                        &mut self.rename,
                        &mut self.rename_deserialize,
                        &mut self.rename_serialize,
                    );
                }
                if meta.path.is_ident("alias") {
                    self.aliases.push(expect_lit(&meta)?);
                    return Ok(());
                }
                if meta.path.is_ident("flatten") {
                    self.flatten = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip") {
                    self.skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip_serializing") {
                    self.skip_serializing = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip_deserializing") {
                    self.skip_deserializing = true;
                    return Ok(());
                }
                if meta.path.is_ident("skip_serializing_if") {
                    if meta.input.parse::<Token![=]>().is_err() {
                        return Err(meta.error("expected `=`"));
                    }
                    self.skip_serializing_if = Some(skip_if_path(meta.input.parse::<LitStr>()?)?);
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    self.default = parse_default_flag(&meta)?;
                    return Ok(());
                }
                Err(meta.error("unsupported serde attribute"))
            })?;

            Ok(())
        }
    }

    #[derive(Default, Clone)]
    pub(crate) enum EnumRepresentation {
        #[default]
        External,
        Internal {
            tag: String,
        },
        Adjacent {
            tag: String,
            content: String,
        },
        Untagged,
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[test]
        fn parses_container_rename() {
            let mut ca = ContainerAttributes::default();

            ca.parse(&syn::parse_quote!(#[serde(rename = "re")]))
                .unwrap();
            assert_eq!(ca.rename, Some("re".to_string()));

            ca.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de"))]))
                .unwrap();
            assert_eq!(ca.rename_deserialize, Some("de".to_string()));

            ca.parse(&syn::parse_quote!(#[serde(rename(serialize = "se"))]))
                .unwrap();
            assert_eq!(ca.rename_serialize, Some("se".to_string()));

            ca.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de2", serialize = "se2"))]))
                .unwrap();
            assert_eq!(ca.rename_deserialize, Some("de2".to_string()));
            assert_eq!(ca.rename_serialize, Some("se2".to_string()));
        }

        #[test]
        fn parses_container_rename_all() {
            let mut ca = ContainerAttributes::default();

            ca.parse(&syn::parse_quote!(#[serde(rename_all = "UPPERCASE")]))
                .unwrap();
            assert!(matches!(ca.rename_all, Some(RenameAll::Uppercase)));

            ca.parse(&syn::parse_quote!(#[serde(rename_all(deserialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_deserialize,
                Some(RenameAll::Uppercase)
            ));

            ca.parse(&syn::parse_quote!(#[serde(rename_all(serialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_serialize,
                Some(RenameAll::Uppercase)
            ));

            ca.parse(&syn::parse_quote!(#[serde(rename_all(deserialize = "lowercase", serialize = "lowercase"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_deserialize,
                Some(RenameAll::Lowercase)
            ));
            assert!(matches!(
                ca.rename_all_serialize,
                Some(RenameAll::Lowercase)
            ));
        }

        #[test]
        fn parses_container_rename_all_fields() {
            let mut ca = ContainerAttributes::default();

            ca.parse(&syn::parse_quote!(#[serde(rename_all_fields = "UPPERCASE")]))
                .unwrap();
            assert!(matches!(ca.rename_all_fields, Some(RenameAll::Uppercase)));

            ca.parse(&syn::parse_quote!(#[serde(rename_all_fields(deserialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_fields_deserialize,
                Some(RenameAll::Uppercase)
            ));

            ca.parse(&syn::parse_quote!(#[serde(rename_all_fields(serialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_fields_serialize,
                Some(RenameAll::Uppercase)
            ));

            ca.parse(&syn::parse_quote!(#[serde(rename_all_fields(deserialize = "lowercase", serialize = "lowercase"))]))
                .unwrap();
            assert!(matches!(
                ca.rename_all_fields_deserialize,
                Some(RenameAll::Lowercase)
            ));
            assert!(matches!(
                ca.rename_all_fields_serialize,
                Some(RenameAll::Lowercase)
            ));
        }

        #[test]
        fn parses_container_enum_representation() {
            let mut ca = ContainerAttributes::default();

            ca.parse(&syn::parse_quote!(#[serde(tag = "t")])).unwrap();
            if let EnumRepresentation::Internal { tag } = &ca.enum_representation {
                assert_eq!(tag, "t");
            } else {
                panic!("failed to parse tag");
            }

            ca.parse(&syn::parse_quote!(#[serde(tag = "t", content = "c")]))
                .unwrap();
            if let EnumRepresentation::Adjacent { tag, content } = &ca.enum_representation {
                assert_eq!(tag, "t");
                assert_eq!(content, "c");
            } else {
                panic!("failed to parse tag and content");
            }

            ca.parse(&syn::parse_quote!(#[serde(untagged)])).unwrap();
            assert!(matches!(
                ca.enum_representation,
                EnumRepresentation::Untagged
            ));
        }

        #[test]
        fn parses_variant_rename() {
            let mut va = VariantAttributes::default();

            va.parse(&syn::parse_quote!(#[serde(rename = "re")]))
                .unwrap();
            assert_eq!(va.rename, Some("re".to_string()));

            va.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de"))]))
                .unwrap();
            assert_eq!(va.rename_deserialize, Some("de".to_string()));

            va.parse(&syn::parse_quote!(#[serde(rename(serialize = "se"))]))
                .unwrap();
            assert_eq!(va.rename_serialize, Some("se".to_string()));

            va.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de2", serialize = "se2"))]))
                .unwrap();
            assert_eq!(va.rename_deserialize, Some("de2".to_string()));
            assert_eq!(va.rename_serialize, Some("se2".to_string()));
        }

        #[test]
        fn parses_variant_rename_all() {
            let mut va = VariantAttributes::default();

            va.parse(&syn::parse_quote!(#[serde(rename_all = "UPPERCASE")]))
                .unwrap();
            assert!(matches!(va.rename_all, Some(RenameAll::Uppercase)));

            va.parse(&syn::parse_quote!(#[serde(rename_all(deserialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                va.rename_all_deserialize,
                Some(RenameAll::Uppercase)
            ));

            va.parse(&syn::parse_quote!(#[serde(rename_all(serialize = "UPPERCASE"))]))
                .unwrap();
            assert!(matches!(
                va.rename_all_serialize,
                Some(RenameAll::Uppercase)
            ));

            va.parse(&syn::parse_quote!(#[serde(rename_all(deserialize = "lowercase", serialize = "lowercase"))]))
                .unwrap();
            assert!(matches!(
                va.rename_all_deserialize,
                Some(RenameAll::Lowercase)
            ));
            assert!(matches!(
                va.rename_all_serialize,
                Some(RenameAll::Lowercase)
            ));
        }

        #[test]
        fn parses_variant_aliases() {
            let mut va = VariantAttributes::default();
            va.parse(&syn::parse_quote!(#[serde(alias = "a")])).unwrap();
            assert_eq!(va.aliases[0], "a");
            va.parse(&syn::parse_quote!(#[serde(alias = "a1", alias = "a2")]))
                .unwrap();
            assert_eq!(va.aliases[1], "a1");
            assert_eq!(va.aliases[2], "a2");
        }

        #[test]
        fn parses_variant_skip() {
            let mut va = VariantAttributes::default();
            va.parse(&syn::parse_quote!(#[serde(skip)])).unwrap();
            assert!(va.skip);
        }

        #[test]
        fn parses_variant_other() {
            let mut va = VariantAttributes::default();
            va.parse(&syn::parse_quote!(#[serde(other)])).unwrap();
            assert!(va.other);
        }

        #[test]
        fn parses_field_rename() {
            let mut fa = FieldAttributes::default();

            fa.parse(&syn::parse_quote!(#[serde(rename = "re")]))
                .unwrap();
            assert_eq!(fa.rename, Some("re".to_string()));

            fa.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de"))]))
                .unwrap();
            assert_eq!(fa.rename_deserialize, Some("de".to_string()));

            fa.parse(&syn::parse_quote!(#[serde(rename(serialize = "se"))]))
                .unwrap();
            assert_eq!(fa.rename_serialize, Some("se".to_string()));

            fa.parse(&syn::parse_quote!(#[serde(rename(deserialize = "de2", serialize = "se2"))]))
                .unwrap();
            assert_eq!(fa.rename_deserialize, Some("de2".to_string()));
            assert_eq!(fa.rename_serialize, Some("se2".to_string()));
        }

        #[test]
        fn parses_field_aliases() {
            let mut fa = FieldAttributes::default();
            fa.parse(&syn::parse_quote!(#[serde(alias = "a")])).unwrap();
            assert_eq!(fa.aliases[0], "a");
            fa.parse(&syn::parse_quote!(#[serde(alias = "a1", alias = "a2")]))
                .unwrap();
            assert_eq!(fa.aliases[1], "a1");
            assert_eq!(fa.aliases[2], "a2");
        }

        #[test]
        fn parses_field_flatten() {
            let mut fa = FieldAttributes::default();
            fa.parse(&syn::parse_quote!(#[serde(flatten)])).unwrap();
            assert!(fa.flatten);
        }

        #[test]
        fn parses_field_skip() {
            let mut fa = FieldAttributes::default();
            fa.parse(&syn::parse_quote!(#[serde(skip)])).unwrap();
            assert!(fa.skip);
        }

        #[test]
        fn parses_field_skip_and_default() {
            let mut fa = FieldAttributes::default();
            fa.parse(&syn::parse_quote!(#[serde(skip_serializing, skip_deserializing, default)]))
                .unwrap();
            assert!(fa.skip_serializing);
            assert!(fa.skip_deserializing);
            assert!(fa.default);

            fa.parse(&syn::parse_quote!(#[serde(skip_serializing_if = "Option::is_none")]))
                .unwrap();
            assert_eq!(fa.skip_serializing_if, Some(SkipIf::OptionIsNone));
        }

        #[test]
        fn parses_deny_unknown_fields_and_container_default() {
            let mut ca = ContainerAttributes::default();
            ca.parse(&syn::parse_quote!(#[serde(deny_unknown_fields, default)]))
                .unwrap();
            assert!(ca.deny_unknown_fields);
            assert!(ca.default);
        }

        #[test]
        fn rejects_unknown_serde_attribute() {
            let mut fa = FieldAttributes::default();
            let err = fa
                .parse(&syn::parse_quote!(#[serde(with = "mod::ser")]))
                .unwrap_err();
            assert!(
                err.to_string().contains("unsupported serde attribute"),
                "{err}"
            );
        }

        #[test]
        fn rejects_default_path() {
            let mut fa = FieldAttributes::default();
            let err = fa
                .parse(&syn::parse_quote!(#[serde(default = "fn_name")]))
                .unwrap_err();
            assert!(err.to_string().contains("no Rust `Default` value"), "{err}");
        }

        #[test]
        fn rejects_unknown_skip_serializing_if() {
            let mut fa = FieldAttributes::default();
            let err = fa
                .parse(&syn::parse_quote!(#[serde(skip_serializing_if = "path::skip")]))
                .unwrap_err();
            assert!(err.to_string().contains("path::skip"), "{err}");
        }

        #[test]
        fn rejects_a_rename_that_is_not_a_string_or_a_pair() {
            let mut va = VariantAttributes::default();
            let err = va.parse(&syn::parse_quote!(#[serde(rename)])).unwrap_err();
            assert!(err.to_string().contains("expected `=` or `(...)`"), "{err}");

            let err = va
                .parse(&syn::parse_quote!(#[serde(rename(serialize))]))
                .unwrap_err();
            assert!(err.to_string().contains("expected `=`"), "{err}");

            let err = va
                .parse(&syn::parse_quote!(#[serde(rename(other = "x"))]))
                .unwrap_err();
            assert!(
                err.to_string().contains("unsupported serde rename option"),
                "{err}"
            );
        }

        #[test]
        fn rejects_a_rename_rule_that_is_not_a_string_or_a_pair() {
            let mut ca = ContainerAttributes::default();
            let err = ca
                .parse(&syn::parse_quote!(#[serde(rename_all)]))
                .unwrap_err();
            assert!(err.to_string().contains("expected `=` or `(...)`"), "{err}");

            let err = ca
                .parse(&syn::parse_quote!(#[serde(rename_all(serialize))]))
                .unwrap_err();
            assert!(err.to_string().contains("expected `=`"), "{err}");

            let err = ca
                .parse(&syn::parse_quote!(#[serde(rename_all(other = "snake_case"))]))
                .unwrap_err();
            assert!(
                err.to_string().contains("unsupported serde rename option"),
                "{err}"
            );
        }

        #[test]
        fn rejects_an_alias_or_tag_without_a_string() {
            let mut va = VariantAttributes::default();
            let err = va.parse(&syn::parse_quote!(#[serde(alias)])).unwrap_err();
            assert!(err.to_string().contains("expected `=`"), "{err}");

            let mut ca = ContainerAttributes::default();
            let err = ca.parse(&syn::parse_quote!(#[serde(tag)])).unwrap_err();
            assert!(err.to_string().contains("expected `=`"), "{err}");
        }

        #[test]
        fn rejects_skip_serializing_if_without_a_path() {
            let mut fa = FieldAttributes::default();
            let err = fa
                .parse(&syn::parse_quote!(#[serde(skip_serializing_if)]))
                .unwrap_err();
            assert!(err.to_string().contains("expected `=`"), "{err}");
        }

        #[test]
        fn rejects_unknown_attributes_on_containers_and_variants() {
            let mut ca = ContainerAttributes::default();
            let err = ca
                .parse(&syn::parse_quote!(#[serde(with = "mod::ser")]))
                .unwrap_err();
            assert!(
                err.to_string().contains("unsupported serde attribute"),
                "{err}"
            );

            let mut va = VariantAttributes::default();
            let err = va.parse(&syn::parse_quote!(#[serde(flatten)])).unwrap_err();
            assert!(
                err.to_string().contains("unsupported serde attribute"),
                "{err}"
            );
        }
    }
}
