use crate::{Elm, ElmDecode, ElmEncode, ElmQuery, ElmQueryField};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Named {
    first: i32,
    second: String,
    e: Enum,
}

#[test]
fn query_struct() {
    super::test_query::<_, Enum>(
        Named {
            first: 123,
            second: "234".to_string(),
            e: Enum::First,
        },
        "?first=123&second=234&e=First",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct ContainsEnum {
    e: Enum,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQueryField)]
enum Enum {
    First,
    Second,
}

#[test]
fn query_enum() {
    super::test_query::<_, Enum>(ContainsEnum { e: Enum::First }, "?e=First");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Thumb {
    width: i32,
    height: i32,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Form {
    flag: bool,
    ratio: f64,
    scale: f32,
    note: Option<String>,
    label: Option<Enum>,
    tags: Vec<Enum>,
    thumbnail: Thumb,
}

fn thumb_deps() -> String {
    format!(
        "{}\n\n{}\n\n{}",
        Thumb::elm_definition().unwrap(),
        Thumb::decoder_definition().unwrap(),
        Thumb::elm_query(),
    )
}

#[test]
fn query_form_omits_nothing_and_repeats_keys() {
    super::test_query_with_deps::<_, Enum>(
        Form {
            flag: true,
            ratio: 1.5,
            scale: 2.0,
            note: None,
            label: None,
            tags: vec![Enum::First, Enum::Second],
            thumbnail: Thumb {
                width: 10,
                height: 20,
            },
        },
        "?flag=true&ratio=1.5&scale=2&tags=First&tags=Second&thumbnail%5Bwidth%5D=10&thumbnail%5Bheight%5D=20",
        &thumb_deps(),
    );
}

#[test]
fn query_form_includes_some() {
    super::test_query_with_deps::<_, Enum>(
        Form {
            flag: false,
            ratio: 0.5,
            scale: 0.25,
            note: Some("hi".to_string()),
            label: Some(Enum::Second),
            tags: vec![],
            thumbnail: Thumb {
                width: 1,
                height: 2,
            },
        },
        "?flag=false&ratio=0.5&scale=0.25&note=hi&label=Second&thumbnail%5Bwidth%5D=1&thumbnail%5Bheight%5D=2",
        &thumb_deps(),
    );
}

#[derive(Elm, ElmQuery)]
#[allow(dead_code)]
struct Wrap<T> {
    item: T,
}

#[test]
fn generic_query_is_polymorphic() {
    let left = Wrap::<i32>::elm_query();
    let right = Wrap::<String>::elm_query();
    assert_eq!(left, right);
    assert!(left.contains("aQuery"), "{left}");
    assert!(left.contains("Url.Builder.string"), "{left}");
    let form = Wrap::<i32>::elm_form();
    assert_eq!(form, Wrap::<String>::elm_form());
    assert!(form.contains("aForm"), "{form}");
    assert!(form.contains("urlDecodeWrap"), "{form}");
}

#[test]
fn export_pulls_nested_query() {
    let mut target = Vec::new();
    crate::export!("Bindings", &mut target, {
        queries: [Form],
    })
    .unwrap();
    let source = String::from_utf8(target).unwrap();
    assert!(source.contains("elmRsQueryThumb"), "{source}");
    assert!(source.contains("queryFieldEncoderEnum"), "{source}");
    assert!(source.contains("queryFieldDecoderEnum"), "{source}");
    assert!(source.contains("urlDecodeForm"), "{source}");
    assert!(source.contains("elmRsFormFind"), "{source}");
    assert!(source.contains("type alias Form"), "{source}");
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Bundle {
    shots: Vec<Thumb>,
    maybe: Option<Thumb>,
    boxed: Box<Thumb>,
    shared: std::rc::Rc<Thumb>,
    atomic: std::sync::Arc<Thumb>,
    pair: [Enum; 2],
    ordered: std::collections::BTreeSet<i32>,
    links: std::collections::LinkedList<i32>,
    ch: char,
    path: std::path::PathBuf,
    ip: std::net::Ipv4Addr,
    sock: std::net::SocketAddr,
    nz: std::num::NonZeroU8,
}

fn sample_bundle() -> Bundle {
    let mut ordered = std::collections::BTreeSet::new();
    ordered.insert(30);
    ordered.insert(10);
    let mut links = std::collections::LinkedList::new();
    links.push_back(3);
    links.push_back(4);
    Bundle {
        shots: vec![
            Thumb {
                width: 1,
                height: 2,
            },
            Thumb {
                width: 3,
                height: 4,
            },
        ],
        maybe: Some(Thumb {
            width: 5,
            height: 6,
        }),
        boxed: Box::new(Thumb {
            width: 7,
            height: 8,
        }),
        shared: std::rc::Rc::new(Thumb {
            width: 9,
            height: 10,
        }),
        atomic: std::sync::Arc::new(Thumb {
            width: 11,
            height: 12,
        }),
        pair: [Enum::First, Enum::Second],
        ordered,
        links,
        ch: 'Z',
        path: std::path::PathBuf::from("dir/file"),
        ip: std::net::Ipv4Addr::new(10, 0, 0, 1),
        sock: std::net::SocketAddr::from(([127, 0, 0, 1], 80)),
        nz: std::num::NonZeroU8::new(8).unwrap(),
    }
}

#[test]
fn query_bundle_repeats_lists_and_peels_pointers() {
    super::test_query_with_deps::<_, Enum>(
        sample_bundle(),
        "?shots%5Bwidth%5D=1&shots%5Bheight%5D=2&shots%5Bwidth%5D=3&shots%5Bheight%5D=4&maybe%5Bwidth%5D=5&maybe%5Bheight%5D=6&boxed%5Bwidth%5D=7&boxed%5Bheight%5D=8&shared%5Bwidth%5D=9&shared%5Bheight%5D=10&atomic%5Bwidth%5D=11&atomic%5Bheight%5D=12&pair=First&pair=Second&ordered=10&ordered=30&links=3&links=4&ch=Z&path=dir%2Ffile&ip=10.0.0.1&sock=127.0.0.1%3A80&nz=8",
        &thumb_deps(),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Sparse {
    shots: Vec<Thumb>,
    maybe: Option<Thumb>,
    note: Option<String>,
}

#[test]
fn query_sparse_omits_empty_list_and_nothing() {
    super::test_query_with_deps::<_, Enum>(
        Sparse {
            shots: vec![],
            maybe: None,
            note: None,
        },
        "",
        &thumb_deps(),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Item<T> {
    item: T,
}

#[test]
fn generic_query_round_trips_the_value() {
    super::test_query_call::<_, Enum>(
        Item { item: 7 },
        "?item=7",
        "",
        "urlEncodeItem String.fromInt value",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Outer<T> {
    inner: Item<T>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct WrappedItem {
    inner: Item<i32>,
}

#[test]
fn concrete_generic_query_stringifies_an_integer() {
    let deps = format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}",
        Item::<i32>::elm_definition().unwrap(),
        Item::<i32>::encoder_definition().unwrap(),
        Item::<i32>::decoder_definition().unwrap(),
        Item::<i32>::elm_query(),
        Item::<i32>::elm_form(),
    );
    let value = WrappedItem {
        inner: Item { item: 7 },
    };
    super::test_query_with_deps::<_, Enum>(&value, "?inner%5Bitem%5D=7", &deps);
    super::test_form_cases(
        vec![(vec![("inner[item]", "7")], Ok(value))],
        &deps,
        "urlDecodeWrappedItem",
    );
}

#[test]
fn generic_query_nests_a_user_type() {
    let deps = format!(
        "{}\n\n{}\n\n{}",
        Item::<i32>::elm_definition().unwrap(),
        Item::<i32>::decoder_definition().unwrap(),
        Item::<i32>::elm_query(),
    );
    super::test_query_call::<_, Enum>(
        Outer {
            inner: Item { item: 7 },
        },
        "?inner%5Bitem%5D=7",
        &deps,
        "urlEncodeOuter String.fromInt value",
    );
}

#[derive(Elm, ElmQuery)]
#[allow(dead_code)]
struct SliceQuery {
    items: &'static [i32],
}

#[test]
fn query_slice_repeats_the_key() {
    let input = format!(
        r#"
import Url.Builder

{definition}

{query}

s =
    Url.Builder.toQuery (urlEncodeSliceQuery {{ items = [ 1, 2 ] }})

"START"
s
"END"

:exit
"#,
        definition = SliceQuery::elm_definition().unwrap(),
        query = SliceQuery::elm_query(),
    );
    assert_eq!(super::run_repl(&input), "?items=1&items=2");
}

#[cfg(feature = "chrono")]
#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct When {
    at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "chrono")]
#[test]
fn query_datetime_is_a_string() {
    let at = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2020, 1, 2, 3, 4, 5).unwrap();
    super::test_query::<_, Enum>(When { at }, "?at=2020-01-02T03%3A04%3A05Z");
}

#[cfg(feature = "uuid")]
#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct IdForm {
    id: uuid::Uuid,
}

#[cfg(feature = "uuid")]
#[test]
fn query_uuid_is_a_string() {
    super::test_query::<_, Enum>(
        IdForm {
            id: uuid::Uuid::nil(),
        },
        "?id=00000000-0000-0000-0000-000000000000",
    );
}

fn enum_form_deps() -> String {
    format!(
        "{}\n\n{}\n\n{}",
        Enum::elm_definition().unwrap(),
        Enum::encoder_definition().unwrap(),
        Enum::query_field_decoder_definition().unwrap(),
    )
}

fn thumb_form_deps() -> String {
    format!(
        "{}\n\n{}\n\n{}\n\n{}",
        enum_form_deps(),
        Thumb::elm_definition().unwrap(),
        Thumb::encoder_definition().unwrap(),
        Thumb::elm_form(),
    )
}

#[test]
fn form_decodes_pairs_back_into_the_struct() {
    super::test_form_cases(
        vec![
            (
                vec![
                    ("flag", "true"),
                    ("ratio", "1.5"),
                    ("scale", "2"),
                    ("tags", "First"),
                    ("tags", "Second"),
                    ("thumbnail[width]", "10"),
                    ("thumbnail[height]", "20"),
                ],
                Ok(Form {
                    flag: true,
                    ratio: 1.5,
                    scale: 2.0,
                    note: None,
                    label: None,
                    tags: vec![Enum::First, Enum::Second],
                    thumbnail: Thumb {
                        width: 10,
                        height: 20,
                    },
                }),
            ),
            (
                vec![
                    ("flag", "false"),
                    ("ratio", "0.5"),
                    ("scale", "0.25"),
                    ("note", "hi"),
                    ("label", "Second"),
                    ("thumbnail[width]", "1"),
                    ("thumbnail[height]", "2"),
                ],
                Ok(Form {
                    flag: false,
                    ratio: 0.5,
                    scale: 0.25,
                    note: Some("hi".to_string()),
                    label: Some(Enum::Second),
                    tags: vec![],
                    thumbnail: Thumb {
                        width: 1,
                        height: 2,
                    },
                }),
            ),
        ],
        &thumb_form_deps(),
        "urlDecodeForm",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Page {
    page: i32,
}

#[test]
fn form_reports_a_missing_or_invalid_field() {
    super::test_form_cases::<Page>(
        vec![
            (vec![], Err("missing page")),
            (vec![("page", "no")], Err("invalid int")),
        ],
        "",
        "urlDecodePage",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct ShotList {
    shots: Vec<Thumb>,
    nums: Vec<i32>,
}

#[test]
fn form_decodes_repeated_nested_keys_in_order() {
    super::test_form_cases(
        vec![(
            vec![
                ("shots[width]", "1"),
                ("shots[height]", "2"),
                ("shots[width]", "3"),
                ("shots[height]", "4"),
                ("nums", "8"),
                ("nums", "9"),
            ],
            Ok(ShotList {
                shots: vec![
                    Thumb {
                        width: 1,
                        height: 2,
                    },
                    Thumb {
                        width: 3,
                        height: 4,
                    },
                ],
                nums: vec![8, 9],
            }),
        )],
        &thumb_form_deps(),
        "urlDecodeShotList",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct Indexed {
    #[rs2elm(index)]
    shots: Vec<Thumb>,
    #[rs2elm(index)]
    nums: Vec<i32>,
    #[rs2elm(index)]
    gaps: Vec<Option<i32>>,
}

#[test]
fn indexed_lists_number_each_element() {
    super::test_query_with_deps::<_, Enum>(
        Indexed {
            shots: vec![
                Thumb {
                    width: 1,
                    height: 2,
                },
                Thumb {
                    width: 3,
                    height: 4,
                },
            ],
            nums: vec![8, 9],
            gaps: vec![Some(1), None, Some(3)],
        },
        "?shots%5B0%5D%5Bwidth%5D=1&shots%5B0%5D%5Bheight%5D=2&shots%5B1%5D%5Bwidth%5D=3&shots%5B1%5D%5Bheight%5D=4&nums%5B0%5D=8&nums%5B1%5D=9&gaps%5B0%5D=1&gaps%5B2%5D=3",
        &thumb_deps(),
    );
    super::test_form_cases(
        vec![(
            vec![
                ("shots[0][width]", "1"),
                ("shots[0][height]", "2"),
                ("shots[1][width]", "3"),
                ("shots[1][height]", "4"),
                ("nums[0]", "8"),
                ("nums[1]", "9"),
                ("gaps[0]", "1"),
                ("gaps[2]", "3"),
            ],
            Ok(Indexed {
                shots: vec![
                    Thumb {
                        width: 1,
                        height: 2,
                    },
                    Thumb {
                        width: 3,
                        height: 4,
                    },
                ],
                nums: vec![8, 9],
                gaps: vec![Some(1), None, Some(3)],
            }),
        )],
        &thumb_form_deps(),
        "urlDecodeIndexed",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct IndexedNums {
    #[rs2elm(index)]
    nums: Vec<i32>,
}

#[test]
fn indexed_list_rejects_a_gap() {
    super::test_form_cases::<IndexedNums>(
        vec![(
            vec![("nums[0]", "1"), ("nums[2]", "3")],
            Err("missing nums[1]"),
        )],
        "",
        "urlDecodeIndexedNums",
    );
}

#[test]
fn generic_form_reads_the_value_parser() {
    super::test_form_cases(
        vec![(vec![("item", "7")], Ok(Item { item: 7 }))],
        "",
        "urlDecodeItem elmRsFormValueInt",
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmDecode, ElmEncode, ElmQuery)]
struct QNode {
    value: i32,
    next: Option<Box<QNode>>,
}

#[test]
fn recursive_form_decodes_a_chain() {
    super::test_form_cases(
        vec![(
            vec![("value", "1"), ("next[value]", "2")],
            Ok(QNode {
                value: 1,
                next: Some(Box::new(QNode {
                    value: 2,
                    next: None,
                })),
            }),
        )],
        "",
        "urlDecodeQNode",
    );
}
