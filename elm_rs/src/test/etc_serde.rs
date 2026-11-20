use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(transparent)]
struct TransparentNamed {
    field: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(transparent)]
struct TransparentNewtype(u8);

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum Other {
    A,
    #[serde(other)]
    B,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all = "UPPERCASE")]
struct RenameStruct {
    uppercase: u8,
    #[serde(rename = "another-field")]
    renamed: u8,
    #[serde(rename(serialize = "se"))]
    rename_for_serialization: u8,
    #[serde(rename(deserialize = "de"))]
    rename_for_deserialization: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all = "UPPERCASE")]
enum RenameEnum {
    Uppercase,
    #[serde(rename = "another-variant")]
    Renamed,
    #[serde(rename(serialize = "se"))]
    RenameForSerialization,
    #[serde(rename(deserialize = "de"))]
    RenameForDeserialization,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Skip {
    #[serde(skip)]
    skipped: u8,
    not_skipped: u8,
}

#[test]
fn transparent_struct() {
    super::test_json(TransparentNamed { field: 0 });
}

#[test]
fn transparent_newtype() {
    super::test_json(TransparentNewtype(0));
}

#[test]
fn other() {
    super::test_json_cases(vec![(r#""other""#.to_string(), Other::B)], "");
}

#[test]
fn rename_struct() {
    super::test_json(RenameStruct {
        uppercase: 0,
        renamed: 0,
        rename_for_serialization: 0,
        rename_for_deserialization: 0,
    })
}

#[test]
fn rename_enum() {
    super::test_json_cases(
        [
            RenameEnum::Uppercase,
            RenameEnum::Renamed,
            RenameEnum::RenameForSerialization,
            RenameEnum::RenameForDeserialization,
        ]
        .into_iter()
        .map(|value| (serde_json::to_string(&value).unwrap(), value))
        .collect(),
        "",
    );
}

#[test]
fn skip() {
    let value = Skip {
        skipped: 0,
        not_skipped: 0,
    };
    let json = serde_json::to_string(&value).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.get("skipped").is_none(), "{json}");
    assert!(parsed.get("not_skipped").is_some(), "{json}");
    super::test_json(value);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct AliasStruct {
    #[serde(alias = "old", alias = "older")]
    name: u8,
}

#[test]
fn field_alias_decodes() {
    super::test_json_cases(
        vec![
            (
                serde_json::to_string(&AliasStruct { name: 3 }).unwrap(),
                AliasStruct { name: 3 },
            ),
            (r#"{"old":3}"#.to_string(), AliasStruct { name: 3 }),
            (r#"{"older":4}"#.to_string(), AliasStruct { name: 4 }),
        ],
        "",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all_fields = "camelCase")]
enum RenameFields {
    One { some_field: i32 },
}

#[test]
fn rename_all_fields() {
    super::test_json(RenameFields::One { some_field: 4 });
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum RenameVariantFields {
    #[serde(rename_all = "camelCase")]
    One { some_field: i32 },
}

#[test]
fn variant_rename_all() {
    super::test_json(RenameVariantFields::One { some_field: 5 });
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum AliasVariant {
    #[serde(alias = "old", alias = "older")]
    One,
}

#[test]
fn variant_alias_decodes() {
    super::test_json_cases(
        vec![
            (
                serde_json::to_string(&AliasVariant::One).unwrap(),
                AliasVariant::One,
            ),
            (r#""old""#.to_string(), AliasVariant::One),
            (r#""older""#.to_string(), AliasVariant::One),
        ],
        "",
    );
}

#[test]
fn other_encodes_its_serialize_name() {
    let json = serde_json::to_string(&Other::B).unwrap();
    assert_eq!(json, "\"B\"");
    let encoder = Other::encoder_definition().unwrap();
    assert!(encoder.contains("Json.Encode.string \"B\""), "{encoder}");
    super::test_json(Other::B);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TupleSkip(#[serde(skip)] u8, u8);

#[test]
fn tuple_skip_omits_the_field() {
    let value = TupleSkip(0, 4);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "[4]", "{json}");
    let definition = TupleSkip::elm_definition().unwrap();
    assert!(definition.contains("TupleSkip (Int)"), "{definition}");
    super::test_json(value);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct SkipSerial {
    #[serde(skip_serializing)]
    hidden: u8,
    shown: u8,
}

#[test]
fn skip_serializing_omits_the_key_and_keeps_the_elm_field() {
    let value = SkipSerial {
        hidden: 7,
        shown: 2,
    };
    assert_eq!(serde_json::to_string(&value).unwrap(), "{\"shown\":2}");
    let definition = SkipSerial::elm_definition().unwrap();
    assert!(definition.contains("hidden"), "{definition}");
    let encoder = SkipSerial::encoder_definition().unwrap();
    assert!(!encoder.contains("\"hidden\""), "{encoder}");
    let decoder = SkipSerial::decoder_type();
    let json = r#"{"hidden":7,"shown":2}"#.replace('"', "\\\"");
    let decoded = super::elm_text::<SkipSerial>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok value ->
            String.fromInt value.hidden ++ "," ++ String.fromInt value.shown
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "7,2", "{decoded}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct SkipDe {
    #[serde(skip_deserializing)]
    hidden: Option<u8>,
    shown: u8,
}

#[test]
fn skip_deserializing_writes_the_field_and_decodes_nothing() {
    let encoder = SkipDe::encoder_type();
    let encoded = super::elm_text::<SkipDe>(&format!(
        r#"    Json.Encode.encode 0 ({encoder} {{ hidden = Just 1, shown = 2 }})"#
    ));
    assert_eq!(encoded, r#"{"hidden":1,"shown":2}"#, "{encoded}");
    let decoder = SkipDe::decoder_type();
    let json = r#"{"hidden":1,"shown":2}"#.replace('"', "\\\"");
    let decoded = super::elm_text::<SkipDe>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok value ->
            case value.hidden of
                Nothing ->
                    "nothing"
                Just _ ->
                    "just"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "nothing", "{decoded}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct DefaultOpt {
    #[serde(default)]
    maybe: Option<u8>,
    n: u8,
}

#[test]
fn default_option_fills_nothing_when_the_field_is_missing() {
    super::test_json(DefaultOpt {
        maybe: Some(3),
        n: 1,
    });
    let decoder = DefaultOpt::decoder_type();
    let json = r#"{"n":1}"#.replace('"', "\\\"");
    let decoded = super::elm_text::<DefaultOpt>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok value ->
            case value.maybe of
                Nothing ->
                    "nothing"
                Just n ->
                    String.fromInt n
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "nothing", "{decoded}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct DefaultUnit {
    #[serde(default)]
    unit: (),
    n: u8,
}

#[test]
fn default_unit_fills_unit_when_the_field_is_missing() {
    super::test_json(DefaultUnit { unit: (), n: 1 });
    let decoder = DefaultUnit::decoder_type();
    let json = r#"{"n":1}"#.replace('"', "\\\"");
    let decoded = super::elm_text::<DefaultUnit>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok value ->
            "ok"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "ok", "{decoded}");
}

#[derive(Debug, Default, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(default)]
struct ContainerDefault {
    maybe: Option<u8>,
    unit: (),
}

#[test]
fn container_default_applies_to_every_field() {
    let decoder = ContainerDefault::decoder_type();
    let decoded = super::elm_text::<ContainerDefault>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{{}}" of
        Ok value ->
            case value.maybe of
                Nothing ->
                    "nothing"
                Just _ ->
                    "just"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "nothing", "{decoded}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct SkipEmpty {
    #[serde(skip_serializing_if = "Option::is_none")]
    maybe: Option<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    items: Vec<u8>,
    #[serde(skip_serializing_if = "String::is_empty")]
    name: String,
    n: u8,
}

#[test]
fn skip_serializing_if_omits_empty_values() {
    super::test_json(SkipEmpty {
        maybe: Some(1),
        items: vec![2],
        name: "a".to_string(),
        n: 3,
    });
    let value = SkipEmpty {
        maybe: None,
        items: vec![],
        name: String::new(),
        n: 1,
    };
    assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"n":1}"#);
    let encoder = SkipEmpty::encoder_type();
    let encoded = super::elm_text::<SkipEmpty>(&format!(
        r#"    Json.Encode.encode 0 ({encoder} {{ maybe = Nothing, items = [], name = "", n = 1 }})"#
    ));
    assert_eq!(encoded, r#"{"n":1}"#, "{encoded}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
struct Deny {
    a: u8,
    #[serde(alias = "old")]
    b: u8,
}

#[test]
fn deny_unknown_fields_rejects_extra_keys_and_accepts_aliases() {
    super::test_json(Deny { a: 1, b: 2 });
    let decoder = Deny::decoder_type();
    let encoder = Deny::encoder_type();
    let alias = r#"{"a":1,"old":2}"#.replace('"', "\\\"");
    let decoded = super::elm_text::<Deny>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{alias}" of
        Ok value ->
            Json.Encode.encode 0 ({encoder} value)
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, r#"{"a":1,"b":2}"#, "{decoded}");
    let extra = r#"{"a":1,"b":2,"extra":3}"#.replace('"', "\\\"");
    let rejected = super::elm_text::<Deny>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{extra}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(rejected.contains("unknown field"), "{rejected}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct FlatInner {
    a: u8,
    b: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct FlatOuter {
    #[serde(flatten)]
    inner: FlatInner,
    c: u8,
}

#[test]
fn flatten_struct_keeps_nested_elm_and_flat_json() {
    let value = FlatOuter {
        inner: FlatInner { a: 1, b: 2 },
        c: 3,
    };
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"a":1,"b":2,"c":3}"#
    );
    let definition = FlatOuter::elm_definition().unwrap();
    assert!(definition.contains("inner : FlatInner"), "{definition}");
    let encoder = FlatOuter::encoder_definition().unwrap();
    assert!(encoder.contains("flatInnerEncoderFields"), "{encoder}");
    super::test_json_with_deps(
        value,
        &format!(
            "{}\n\n{}\n\n{}",
            FlatInner::elm_definition().unwrap(),
            FlatInner::encoder_definition().unwrap(),
            FlatInner::decoder_definition().unwrap(),
        ),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct FlatMap {
    n: i32,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, i32>,
}

#[test]
fn flatten_map_merges_keys() {
    let mut extra = std::collections::HashMap::new();
    extra.insert("a".to_string(), 1);
    let value = FlatMap { n: 2, extra };
    assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"n":2,"a":1}"#);
    super::test_json(value);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
struct DenyFlat {
    #[serde(flatten)]
    inner: FlatInner,
    c: u8,
}

#[test]
fn flatten_with_deny_unknown_fields_accepts_inner_keys() {
    let value = DenyFlat {
        inner: FlatInner { a: 1, b: 2 },
        c: 3,
    };
    super::test_json_with_deps(
        value,
        &format!(
            "{}\n\n{}\n\n{}",
            FlatInner::elm_definition().unwrap(),
            FlatInner::encoder_definition().unwrap(),
            FlatInner::decoder_definition().unwrap(),
        ),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[rs2elm(rename = "Picture")]
struct RenamedDrawing {
    #[rs2elm(rename = "fileName")]
    filename: String,
}

#[test]
fn elm_rename_changes_elm_names_and_keeps_json_names() {
    let definition = RenamedDrawing::elm_definition().unwrap();
    assert!(definition.contains("type alias Picture"), "{definition}");
    assert!(definition.contains("fileName : String"), "{definition}");
    let value = RenamedDrawing {
        filename: "a.png".to_string(),
    };
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"filename":"a.png"}"#
    );
    super::test_json(value);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[allow(dead_code)]
enum Visibility {
    Shown,
    #[serde(skip)]
    Hidden,
}

#[test]
fn skipped_variant_is_absent_from_elm() {
    let definition = Visibility::elm_definition().unwrap();
    assert!(definition.contains("= Shown"), "{definition}");
    assert!(!definition.contains("Hidden"), "{definition}");
    super::test_json(Visibility::Shown);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all = "kebab-case")]
struct KebabStruct {
    some_field: u8,
}

#[test]
fn kebab_case_renames_fields() {
    super::test_json(KebabStruct { some_field: 1 });
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum AliasShapes {
    #[serde(alias = "old_new")]
    Newtype(u8),
    #[serde(alias = "old_tuple")]
    Tuple(u8, u8),
    #[serde(alias = "legacy")]
    Struct { n: u8 },
}

#[test]
fn variant_alias_decodes_an_externally_tagged_payload() {
    super::test_json_cases(
        vec![
            (
                serde_json::to_string(&AliasShapes::Newtype(1)).unwrap(),
                AliasShapes::Newtype(1),
            ),
            (r#"{"old_new":1}"#.to_string(), AliasShapes::Newtype(1)),
            (
                serde_json::to_string(&AliasShapes::Tuple(2, 3)).unwrap(),
                AliasShapes::Tuple(2, 3),
            ),
            (
                r#"{"old_tuple":[2,3]}"#.to_string(),
                AliasShapes::Tuple(2, 3),
            ),
            (
                serde_json::to_string(&AliasShapes::Struct { n: 4 }).unwrap(),
                AliasShapes::Struct { n: 4 },
            ),
            (
                r#"{"legacy":{"n":4}}"#.to_string(),
                AliasShapes::Struct { n: 4 },
            ),
        ],
        "",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t")]
enum AliasInternal {
    #[serde(alias = "old")]
    Struct { n: u8 },
}

#[test]
fn variant_alias_decodes_an_internal_struct() {
    super::test_json_cases(
        vec![(
            r#"{"t":"old","n":3}"#.to_string(),
            AliasInternal::Struct { n: 3 },
        )],
        "",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", content = "c")]
enum AliasAdjacent {
    #[serde(alias = "old")]
    Newtype(u8),
    #[serde(alias = "aged")]
    Pair(u8, u8),
}

#[test]
fn variant_alias_decodes_adjacent_payloads() {
    super::test_json_cases(
        vec![
            (
                r#"{"t":"old","c":4}"#.to_string(),
                AliasAdjacent::Newtype(4),
            ),
            (
                r#"{"t":"aged","c":[1,2]}"#.to_string(),
                AliasAdjacent::Pair(1, 2),
            ),
        ],
        "",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct OptionalField {
    maybe: Option<u8>,
    n: u8,
}

#[test]
fn a_missing_option_field_decodes_as_nothing() {
    super::test_json_cases(
        vec![
            (
                serde_json::to_string(&OptionalField {
                    maybe: Some(2),
                    n: 1,
                })
                .unwrap(),
                OptionalField {
                    maybe: Some(2),
                    n: 1,
                },
            ),
            (
                r#"{"n":1}"#.to_string(),
                OptionalField { maybe: None, n: 1 },
            ),
            (
                r#"{"maybe":null,"n":1}"#.to_string(),
                OptionalField { maybe: None, n: 1 },
            ),
        ],
        "",
    );
    let decoder = OptionalField::decoder_type();
    let bad = r#"{"maybe":"no","n":1}"#.replace('"', "\\\"");
    let rejected = super::elm_text::<OptionalField>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{bad}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(rejected.contains("Problem"), "{rejected}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TupleSkipDe(#[serde(skip_deserializing)] Option<u8>, u8);

#[test]
fn tuple_skip_deserializing_reads_the_later_element() {
    assert_eq!(
        serde_json::to_string(&TupleSkipDe(Some(1), 4)).unwrap(),
        "[1,4]"
    );
    let encoder = TupleSkipDe::encoder_type();
    let encoded = super::elm_text::<TupleSkipDe>(&format!(
        r#"    Json.Encode.encode 0 ({encoder} (TupleSkipDe (Just 1) 4))"#
    ));
    assert_eq!(encoded, "[1,4]", "{encoded}");
    let decoder = TupleSkipDe::decoder_type();
    let decoded = super::elm_text::<TupleSkipDe>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "[4]" of
        Ok (TupleSkipDe hidden shown) ->
            (case hidden of
                Nothing ->
                    "nothing"
                Just _ ->
                    "just"
            )
                ++ ","
                ++ String.fromInt shown
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(decoded, "nothing,4", "{decoded}");
    let extra = super::elm_text::<TupleSkipDe>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "[1,4]" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(extra.contains("expected a list of length 1"), "{extra}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TupleDefault(u8, #[serde(default)] Option<u8>);

#[test]
fn tuple_default_fills_a_missing_tail() {
    super::test_json_cases(
        vec![
            (r#"[4]"#.to_string(), TupleDefault(4, None)),
            (r#"[4,1]"#.to_string(), TupleDefault(4, Some(1))),
        ],
        "",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TupleSkipIf(
    #[serde(skip_serializing_if = "Option::is_none")] Option<u8>,
    u8,
);

#[test]
fn tuple_skip_serializing_if_omits_the_element() {
    assert_eq!(serde_json::to_string(&TupleSkipIf(None, 4)).unwrap(), "[4]");
    let encoder = TupleSkipIf::encoder_type();
    let encoded = super::elm_text::<TupleSkipIf>(&format!(
        r#"    Json.Encode.encode 0 ({encoder} (TupleSkipIf Nothing 4))"#
    ));
    assert_eq!(encoded, "[4]", "{encoded}");
    super::test_json(TupleSkipIf(Some(1), 4));
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all(serialize = "UPPERCASE", deserialize = "lowercase"))]
struct SplitNames {
    field_name: u8,
}

#[test]
fn split_rename_all_follows_serde() {
    super::test_json(SplitNames { field_name: 1 });
}

fn codecs<T: Elm + ElmEncode + ElmDecode>() -> String {
    format!(
        "{}\n\n{}\n\n{}",
        T::elm_definition().unwrap(),
        T::encoder_definition().unwrap(),
        T::decoder_definition().unwrap(),
    )
}

fn elm_decode_text<T: Elm + ElmEncode + ElmDecode>(deps: &str, json: &str) -> String {
    let decoder = T::decoder_type();
    let escaped = json.replace('\\', "\\\\").replace('"', "\\\"");
    super::elm_text_with_deps::<T>(
        deps,
        &format!(
            r#"    case Json.Decode.decodeString {decoder} "{escaped}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
        ),
    )
}

fn elm_reencode<T: Elm + ElmEncode + ElmDecode>(deps: &str, json: &str) -> String {
    let decoder = T::decoder_type();
    let encoder = T::encoder_type();
    let escaped = json.replace('\\', "\\\\").replace('"', "\\\"");
    super::elm_text_with_deps::<T>(
        deps,
        &format!(
            r#"    case Json.Decode.decodeString {decoder} "{escaped}" of
        Ok value ->
            Json.Encode.encode 0 ({encoder} value)
        Err err ->
            Json.Decode.errorToString err"#
        ),
    )
}

fn assert_both_fail<
    T: Elm + ElmEncode + ElmDecode + serde::de::DeserializeOwned + std::fmt::Debug,
>(
    deps: &str,
    json: &str,
    message: &str,
) {
    let rust = serde_json::from_str::<T>(json).unwrap_err().to_string();
    assert!(rust.contains(message), "{rust}");
    let elm = elm_decode_text::<T>(deps, json);
    assert!(elm.contains(message), "{elm}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Steal {
    a: u8,
    #[serde(flatten)]
    inner: FlatInner,
}

#[test]
fn a_parent_field_keeps_a_key_the_flattened_struct_also_needs() {
    let deps = &codecs::<FlatInner>();
    assert_both_fail::<Steal>(deps, r#"{"a":1,"b":2}"#, "missing field `a`");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct RightFlat {
    a: u8,
    c: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TwoFlat {
    #[serde(flatten)]
    left: FlatInner,
    #[serde(flatten)]
    right: RightFlat,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct OnlyC {
    c: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct BothFlat {
    #[serde(flatten)]
    left: FlatInner,
    #[serde(flatten)]
    right: OnlyC,
}

#[test]
fn flattened_structs_consume_keys_in_order() {
    let overlap = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<RightFlat>());
    assert_both_fail::<TwoFlat>(&overlap, r#"{"a":1,"b":2,"c":4}"#, "missing field `a`");
    let share = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<OnlyC>());
    let value = BothFlat {
        left: FlatInner { a: 1, b: 2 },
        right: OnlyC { c: 4 },
    };
    assert_eq!(
        serde_json::from_str::<BothFlat>(r#"{"a":1,"b":2,"c":4}"#).unwrap(),
        value
    );
    super::test_json_with_deps(value, &share);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct AliasClash {
    #[serde(alias = "old")]
    b: u8,
    #[serde(flatten)]
    inner: FlatInner,
}

#[test]
fn a_field_and_its_alias_are_a_duplicate_when_both_are_present() {
    let deps = &codecs::<FlatInner>();
    assert_both_fail::<AliasClash>(deps, r#"{"old":1,"b":2}"#, "duplicate field `b`");
}

#[test]
fn deny_unknown_fields_rejects_keys_a_flattened_struct_did_not_consume() {
    let deps = &codecs::<FlatInner>();
    assert_both_fail::<DenyFlat>(
        deps,
        r#"{"a":1,"b":2,"c":3,"extra":4}"#,
        "unknown field `extra`",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
struct DenyMap {
    n: i32,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, i32>,
}

#[test]
fn deny_unknown_fields_rejects_keys_captured_by_a_flattened_map() {
    assert_both_fail::<DenyMap>("", r#"{"n":2,"a":1}"#, "unknown field `a`");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
struct InnerDeny {
    a: u8,
    b: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct ParentLoose {
    #[serde(flatten)]
    inner: InnerDeny,
    c: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
struct ParentStrict {
    #[serde(flatten)]
    inner: InnerDeny,
    c: u8,
}

#[test]
fn an_inner_deny_unknown_fields_does_not_see_the_parent_object() {
    let deps = &codecs::<InnerDeny>();
    let value = ParentLoose {
        inner: InnerDeny { a: 1, b: 2 },
        c: 3,
    };
    super::test_json_with_deps(value, deps);
    let extra = r#"{"a":1,"b":2,"c":3,"extra":4}"#;
    assert!(serde_json::from_str::<ParentLoose>(extra).is_ok());
    assert_eq!(
        elm_reencode::<ParentLoose>(deps, extra),
        r#"{"a":1,"b":2,"c":3}"#
    );
    super::test_json_with_deps(
        ParentStrict {
            inner: InnerDeny { a: 1, b: 2 },
            c: 3,
        },
        deps,
    );
    assert_both_fail::<ParentStrict>(deps, extra, "unknown field `extra`");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct StructThenMap {
    #[serde(flatten)]
    inner: FlatInner,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, i32>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct MapThenStruct {
    #[serde(flatten)]
    extra: std::collections::HashMap<String, i32>,
    #[serde(flatten)]
    inner: FlatInner,
}

#[test]
fn flatten_order_decides_which_value_keeps_a_key() {
    let deps = &codecs::<FlatInner>();
    let json = r#"{"a":1,"b":2,"c":3}"#;
    let mut after = std::collections::HashMap::new();
    after.insert("c".to_string(), 3);
    let struct_first = StructThenMap {
        inner: FlatInner { a: 1, b: 2 },
        extra: after,
    };
    assert_eq!(
        serde_json::from_str::<StructThenMap>(json).unwrap(),
        struct_first
    );
    super::test_json_cases(vec![(json.to_string(), struct_first)], deps);
    let mut all = std::collections::HashMap::new();
    all.insert("a".to_string(), 1);
    all.insert("b".to_string(), 2);
    all.insert("c".to_string(), 3);
    let map_first = MapThenStruct {
        extra: all,
        inner: FlatInner { a: 1, b: 2 },
    };
    assert_eq!(
        serde_json::from_str::<MapThenStruct>(json).unwrap(),
        map_first
    );
    super::test_json_cases(vec![(json.to_string(), map_first)], deps);
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct WrapInner(FlatInner);

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct WrapOuter {
    #[serde(flatten)]
    inner: WrapInner,
    c: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(transparent)]
struct TransInner {
    inner: FlatInner,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TransOuter {
    #[serde(flatten)]
    inner: TransInner,
    c: u8,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct WrapMap(std::collections::HashMap<String, i32>);

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct WrapMapOuter {
    n: i32,
    #[serde(flatten)]
    extra: WrapMap,
}

#[test]
fn flatten_forwards_newtype_and_transparent_objects() {
    let inner = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<WrapInner>());
    super::test_json_with_deps(
        WrapOuter {
            inner: WrapInner(FlatInner { a: 1, b: 2 }),
            c: 3,
        },
        &inner,
    );
    let transparent = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<TransInner>());
    super::test_json_with_deps(
        TransOuter {
            inner: TransInner {
                inner: FlatInner { a: 1, b: 2 },
            },
            c: 3,
        },
        &transparent,
    );
    let mut extra = std::collections::HashMap::new();
    extra.insert("a".to_string(), 1);
    super::test_json_with_deps(
        WrapMapOuter {
            n: 2,
            extra: WrapMap(extra),
        },
        &codecs::<WrapMap>(),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtSkip {
    New(#[serde(skip_deserializing)] Option<u8>),
    Keep(u8),
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", content = "c")]
enum AdjSkip {
    New(#[serde(skip_deserializing)] Option<u8>),
    Keep(u8),
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(untagged)]
enum UntaggedSkip {
    New(#[serde(skip_deserializing)] Option<u8>),
    Num(u8),
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtUnitSkip {
    New(#[serde(skip_deserializing)] ()),
    Keep(u8),
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", content = "c")]
enum AdjUnitSkip {
    New(#[serde(skip_deserializing)] ()),
    Keep(u8),
}

fn encode_variant<T: Elm + ElmEncode + ElmDecode>(elm_value: &str) -> String {
    let encoder = T::encoder_type();
    super::elm_text::<T>(&format!(
        r#"    Json.Encode.encode 0 ({encoder} {elm_value})"#
    ))
}

fn match_new<T: Elm + ElmEncode + ElmDecode>(json: &str) -> String {
    let decoder = T::decoder_type();
    let escaped = json.replace('\\', "\\\\").replace('"', "\\\"");
    super::elm_text::<T>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{escaped}" of
        Ok (New hidden) ->
            case hidden of
                Nothing ->
                    "nothing"
                Just _ ->
                    "just"
        Ok _ ->
            "other"
        Err err ->
            Json.Decode.errorToString err"#
    ))
}

#[test]
fn skip_deserializing_reads_an_external_newtype_as_a_unit() {
    super::test_json(ExtSkip::New(None));
    super::test_json(ExtSkip::Keep(4));
    assert_eq!(
        encode_variant::<ExtSkip>("(New (Just 1))"),
        serde_json::to_string(&ExtSkip::New(Some(1))).unwrap()
    );
    assert_eq!(match_new::<ExtSkip>(r#""New""#), "nothing");
    assert_eq!(match_new::<ExtSkip>(r#"{"New":null}"#), "nothing");
    let rust = serde_json::from_str::<ExtSkip>(r#"{"New":1}"#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("expected unit"), "{rust}");
    let elm = elm_decode_text::<ExtSkip>("", r#"{"New":1}"#);
    assert!(elm.contains("Expecting null"), "{elm}");
    super::test_json(ExtUnitSkip::New(()));
    let decoder = ExtUnitSkip::decoder_type();
    let unit = super::elm_text::<ExtUnitSkip>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "\"New\"" of
        Ok (New ()) ->
            "unit"
        Ok _ ->
            "other"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert_eq!(unit, "unit", "{unit}");
}

#[test]
fn skip_deserializing_reads_an_adjacent_newtype_as_a_unit() {
    super::test_json(AdjSkip::New(None));
    super::test_json(AdjSkip::Keep(4));
    assert_eq!(
        encode_variant::<AdjSkip>("(New (Just 1))"),
        serde_json::to_string(&AdjSkip::New(Some(1))).unwrap()
    );
    assert_eq!(match_new::<AdjSkip>(r#"{"t":"New"}"#), "nothing");
    assert_eq!(match_new::<AdjSkip>(r#"{"t":"New","c":null}"#), "nothing");
    let rust = serde_json::from_str::<AdjSkip>(r#"{"t":"New","c":1}"#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("expected unit"), "{rust}");
    let elm = elm_decode_text::<AdjSkip>("", r#"{"t":"New","c":1}"#);
    assert!(elm.contains("Expecting null"), "{elm}");
    super::test_json(AdjUnitSkip::New(()));
    super::test_json(AdjUnitSkip::Keep(4));
    let rust = serde_json::from_str::<AdjUnitSkip>(r#"{"t":"New"}"#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("missing field `c`"), "{rust}");
    let elm = elm_decode_text::<AdjUnitSkip>("", r#"{"t":"New"}"#);
    assert!(
        elm.contains("Expecting an OBJECT with a field named `c`"),
        "{elm}"
    );
}

#[test]
fn skip_deserializing_reads_an_untagged_newtype_as_null() {
    super::test_json(UntaggedSkip::New(None));
    super::test_json(UntaggedSkip::Num(1));
    assert_eq!(
        encode_variant::<UntaggedSkip>("(New (Just 1))"),
        serde_json::to_string(&UntaggedSkip::New(Some(1))).unwrap()
    );
    let rust = serde_json::from_str::<UntaggedSkip>(r#""New""#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("did not match"), "{rust}");
    let elm = elm_decode_text::<UntaggedSkip>("", r#""New""#);
    assert!(elm.contains("Expecting null"), "{elm}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtFlat {
    Held {
        n: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
    Other {
        n: u8,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t")]
enum InternalFlat {
    Held {
        n: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", content = "c")]
enum AdjFlat {
    Held {
        n: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(untagged)]
enum UntaggedFlat {
    Held {
        n: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
enum ExtDenyFlat {
    Held {
        #[serde(flatten)]
        inner: FlatInner,
        c: u8,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", deny_unknown_fields)]
enum InternalDenyFlat {
    Held {
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t", content = "c", deny_unknown_fields)]
enum AdjDenyFlat {
    Held {
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(untagged, deny_unknown_fields)]
enum UntaggedDenyFlat {
    Held {
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtStealFlat {
    Held {
        a: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtTwoFlat {
    Held {
        #[serde(flatten)]
        left: FlatInner,
        #[serde(flatten)]
        right: RightFlat,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtBothFlat {
    Held {
        #[serde(flatten)]
        left: FlatInner,
        #[serde(flatten)]
        right: OnlyC,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtAliasFlat {
    Held {
        #[serde(alias = "old")]
        b: u8,
        #[serde(flatten)]
        inner: FlatInner,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t")]
enum InternalMapFlat {
    Held {
        n: i32,
        #[serde(flatten)]
        extra: std::collections::HashMap<String, i32>,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(deny_unknown_fields)]
enum ExtDenyMapFlat {
    Held {
        n: i32,
        #[serde(flatten)]
        extra: std::collections::HashMap<String, i32>,
    },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ExtWrapFlat {
    Held {
        #[serde(flatten)]
        inner: WrapInner,
        c: u8,
    },
}

fn flat_deps() -> String {
    codecs::<FlatInner>()
}

#[test]
fn flatten_inside_an_enum_struct_variant_merges_keys() {
    let deps = flat_deps();
    let held = ExtFlat::Held {
        n: 1,
        inner: FlatInner { a: 2, b: 3 },
    };
    assert_eq!(
        serde_json::to_string(&held).unwrap(),
        r#"{"Held":{"n":1,"a":2,"b":3}}"#
    );
    let encoder = ExtFlat::encoder_definition().unwrap();
    assert!(encoder.contains("flatInnerEncoderFields"), "{encoder}");
    assert!(encoder.contains("List.concat"), "{encoder}");
    super::test_json_cases(
        vec![
            (serde_json::to_string(&held).unwrap(), held),
            (
                serde_json::to_string(&ExtFlat::Other { n: 4 }).unwrap(),
                ExtFlat::Other { n: 4 },
            ),
        ],
        &deps,
    );
    super::test_json_with_deps(
        InternalFlat::Held {
            n: 1,
            inner: FlatInner { a: 2, b: 3 },
        },
        &deps,
    );
    super::test_json_with_deps(
        AdjFlat::Held {
            n: 1,
            inner: FlatInner { a: 2, b: 3 },
        },
        &deps,
    );
    super::test_json_with_deps(
        UntaggedFlat::Held {
            n: 1,
            inner: FlatInner { a: 2, b: 3 },
        },
        &deps,
    );
    let internal = InternalFlat::encoder_definition().unwrap();
    assert!(internal.contains("flatInnerEncoderFields"), "{internal}");
    let adjacent = AdjFlat::encoder_definition().unwrap();
    assert!(adjacent.contains("List.concat"), "{adjacent}");
    let untagged = UntaggedFlat::encoder_definition().unwrap();
    assert!(untagged.contains("List.concat"), "{untagged}");
}

#[test]
fn a_variant_field_keeps_a_key_the_flattened_struct_also_needs() {
    let deps = flat_deps();
    assert_both_fail::<ExtFlat>(&deps, r#"{"Held":{"n":1,"b":3}}"#, "missing field `a`");
    assert_both_fail::<InternalFlat>(&deps, r#"{"t":"Held","n":1,"a":2}"#, "missing field `b`");
    assert_both_fail::<AdjFlat>(
        &deps,
        r#"{"t":"Held","c":{"n":1,"a":2}}"#,
        "missing field `b`",
    );
    assert_both_fail::<ExtStealFlat>(&deps, r#"{"Held":{"a":1,"b":2}}"#, "missing field `a`");
}

#[test]
fn variant_deny_unknown_fields_runs_after_a_flattened_struct_consumes_keys() {
    let deps = flat_deps();
    super::test_json_with_deps(
        ExtDenyFlat::Held {
            inner: FlatInner { a: 1, b: 2 },
            c: 3,
        },
        &deps,
    );
    assert_both_fail::<ExtDenyFlat>(
        &deps,
        r#"{"Held":{"a":1,"b":2,"c":3,"extra":4}}"#,
        "unknown field `extra`",
    );
    super::test_json_with_deps(
        InternalDenyFlat::Held {
            inner: FlatInner { a: 1, b: 2 },
        },
        &deps,
    );
    assert_both_fail::<InternalDenyFlat>(
        &deps,
        r#"{"t":"Held","a":1,"b":2,"extra":4}"#,
        "unknown field `extra`",
    );
    super::test_json_with_deps(
        AdjDenyFlat::Held {
            inner: FlatInner { a: 1, b: 2 },
        },
        &deps,
    );
    assert_both_fail::<AdjDenyFlat>(
        &deps,
        r#"{"t":"Held","c":{"a":1,"b":2,"extra":4}}"#,
        "unknown field `extra`",
    );
    super::test_json_with_deps(
        UntaggedDenyFlat::Held {
            inner: FlatInner { a: 1, b: 2 },
        },
        &deps,
    );
}

#[test]
fn flattened_structs_inside_a_variant_consume_keys_in_order() {
    let overlap = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<RightFlat>());
    assert_both_fail::<ExtTwoFlat>(
        &overlap,
        r#"{"Held":{"a":1,"b":2,"c":4}}"#,
        "missing field `a`",
    );
    let share = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<OnlyC>());
    super::test_json_with_deps(
        ExtBothFlat::Held {
            left: FlatInner { a: 1, b: 2 },
            right: OnlyC { c: 4 },
        },
        &share,
    );
}

#[test]
fn a_variant_field_and_its_alias_are_a_duplicate_when_both_are_present() {
    assert_both_fail::<ExtAliasFlat>(
        &flat_deps(),
        r#"{"Held":{"old":1,"b":2}}"#,
        "duplicate field `b`",
    );
}

#[test]
fn an_internal_tag_is_not_part_of_a_flattened_map() {
    let mut extra = std::collections::HashMap::new();
    extra.insert("a".to_string(), 1);
    let value = InternalMapFlat::Held { n: 2, extra };
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        r#"{"t":"Held","n":2,"a":1}"#
    );
    super::test_json(value);
    assert_both_fail::<ExtDenyMapFlat>("", r#"{"Held":{"n":2,"a":1}}"#, "unknown field `a`");
}

#[test]
fn flatten_inside_a_variant_forwards_a_newtype_object() {
    let deps = format!("{}\n\n{}", codecs::<FlatInner>(), codecs::<WrapInner>());
    super::test_json_with_deps(
        ExtWrapFlat::Held {
            inner: WrapInner(FlatInner { a: 1, b: 2 }),
            c: 3,
        },
        &deps,
    );
}

#[test]
fn an_untagged_flatten_failure_reports_each_side() {
    let deps = flat_deps();
    let rust = serde_json::from_str::<UntaggedFlat>(r#"{"n":1,"a":2}"#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("did not match"), "{rust}");
    let elm = elm_decode_text::<UntaggedFlat>(&deps, r#"{"n":1,"a":2}"#);
    assert!(elm.contains("missing field `b`"), "{elm}");
    let rust = serde_json::from_str::<UntaggedDenyFlat>(r#"{"a":1,"b":2,"extra":4}"#)
        .unwrap_err()
        .to_string();
    assert!(rust.contains("did not match"), "{rust}");
    let elm = elm_decode_text::<UntaggedDenyFlat>(&deps, r#"{"a":1,"b":2,"extra":4}"#);
    assert!(elm.contains("unknown field `extra`"), "{elm}");
}
