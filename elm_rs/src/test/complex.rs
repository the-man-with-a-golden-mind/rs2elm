use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Elm, ElmEncode, ElmDecode, Serialize, Deserialize, PartialEq)]
enum Enum1<T> {
    Unit11,
    Unit12,
    Newtype11(T),
    Newtype12(T),
    Tuple11(T, T),
    Tuple12(T, T),
    Named11 { t: T },
    Named12 { t: T },
}

#[derive(Debug, Elm, ElmEncode, ElmDecode, Serialize, Deserialize, PartialEq)]
enum Enum2<T> {
    Unit21,
    Unit22,
    Newtype21(T),
    Newtype22(T),
    Tuple21(T, T),
    Tuple22(T, T),
    Named21 { t: T },
    Named22 { t: T },
}

#[derive(Debug, Elm, ElmEncode, ElmDecode, Serialize, Deserialize, PartialEq)]
struct Struct<T> {
    unit: Enum1<T>,
    newtype: Enum1<T>,
    tuple: Enum1<T>,
    named: Enum1<T>,
    named_unit: Enum2<Enum1<T>>,
    named_newtype: Enum2<Enum1<T>>,
    named_tuple: Enum2<Enum1<T>>,
    named_named: Enum2<Enum1<T>>,
}

#[test]
fn complex() {
    assert_eq!(
        Enum1::<u8>::elm_definition(),
        Enum1::<String>::elm_definition()
    );
    let definition = Enum1::<u8>::elm_definition().unwrap();
    assert!(definition.contains("type Enum1 a"), "{definition}");
    let encoder = Enum1::<u8>::encoder_definition().unwrap();
    assert!(encoder.contains("aEncoder"), "{encoder}");
    super::test_json_with_deps(
        Struct {
            unit: Enum1::Unit11,
            newtype: Enum1::Newtype11(vec![1, 2, 3, 4]),
            tuple: Enum1::Tuple11(vec![1, 2, 3, 4], vec![1, 2, 3, 4]),
            named: Enum1::Named11 {
                t: vec![1, 2, 3, 4],
            },
            named_unit: Enum2::Named21 { t: Enum1::Unit11 },
            named_newtype: Enum2::Named21 {
                t: Enum1::Newtype11(vec![1, 2, 3, 4]),
            },
            named_tuple: Enum2::Named21 {
                t: Enum1::Tuple11(vec![1, 2, 3, 4], vec![1, 2, 3, 4]),
            },
            named_named: Enum2::Named21 {
                t: Enum1::Named11 {
                    t: vec![1, 2, 3, 4],
                },
            },
        },
        &format!(
            "\
{}

{}

{}

{}

{}

{}
",
            Enum1::<Vec<usize>>::elm_definition().unwrap(),
            Enum1::<Vec<usize>>::encoder_definition().unwrap(),
            Enum1::<Vec<usize>>::decoder_definition().unwrap(),
            Enum2::<Enum1<Vec<usize>>>::elm_definition().unwrap(),
            Enum2::<Enum1<Vec<usize>>>::encoder_definition().unwrap(),
            Enum2::<Enum1<Vec<usize>>>::decoder_definition().unwrap(),
        ),
    )
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Elm, ElmEncode, ElmDecode)]
struct Slot<T> {
    value: T,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Elm, ElmEncode, ElmDecode)]
struct Hold<T> {
    value: T,
    maybe_some: Option<T>,
    maybe_none: Option<T>,
    items: Vec<T>,
    pair: [T; 2],
    boxed: Box<T>,
    text: std::borrow::Cow<'static, str>,
    rows: std::collections::HashMap<String, T>,
    outcome_ok: Result<T, String>,
    outcome_err: Result<T, String>,
    slot: Slot<T>,
    ghost: std::marker::PhantomData<T>,
}

#[test]
fn generic_containers_round_trip() {
    let left = Hold::<i32>::elm_definition();
    let right = Hold::<String>::elm_definition();
    assert_eq!(left, right);
    let definition = left.unwrap();
    assert!(definition.contains("type alias Hold a"), "{definition}");
    assert!(definition.contains("slot : Slot a"), "{definition}");
    let mut rows = std::collections::HashMap::new();
    rows.insert("k".to_string(), 9);
    super::test_json_with_deps(
        Hold {
            value: 10,
            maybe_some: Some(8),
            maybe_none: None,
            items: vec![6, 7],
            pair: [4, 5],
            boxed: Box::new(9),
            text: std::borrow::Cow::Borrowed("elm"),
            rows,
            outcome_ok: Ok(11),
            outcome_err: Err("no".to_string()),
            slot: Slot { value: 12 },
            ghost: std::marker::PhantomData,
        },
        &format!(
            "{}\n\n{}\n\n{}\n",
            Slot::<i32>::elm_definition().unwrap(),
            Slot::<i32>::encoder_definition().unwrap(),
            Slot::<i32>::decoder_definition().unwrap(),
        ),
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Elm, ElmEncode, ElmDecode)]
struct PairHold<T> {
    pair: [T; 2],
}

#[test]
fn generic_array_rejects_the_wrong_length() {
    let decoder = PairHold::<i32>::decoder_type();
    let json = r#"{"pair":[1]}"#.replace('"', "\\\"");
    let rejected = super::elm_text::<PairHold<i32>>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(
        rejected.contains("expected a list of length 2"),
        "{rejected}"
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Elm, ElmEncode, ElmDecode)]
enum Tree<T> {
    Leaf(T),
    Branch(Box<Tree<T>>, Box<Tree<T>>),
}

#[test]
fn generic_tree_round_trips() {
    let definition = Tree::<i32>::elm_definition().unwrap();
    assert!(definition.contains("type Tree a"), "{definition}");
    let decoder = Tree::<i32>::decoder_definition().unwrap();
    assert!(decoder.contains("Json.Decode.lazy"), "{decoder}");
    assert_eq!(
        Tree::<i32>::elm_definition(),
        Tree::<String>::elm_definition()
    );
    let branch = Tree::Branch(
        Box::new(Tree::Leaf(2)),
        Box::new(Tree::Branch(
            Box::new(Tree::Leaf(3)),
            Box::new(Tree::Leaf(4)),
        )),
    );
    super::test_json_cases(
        [Tree::Leaf(1), branch]
            .into_iter()
            .map(|value| (serde_json::to_string(&value).unwrap(), value))
            .collect(),
        "",
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Elm, ElmEncode, ElmDecode)]
struct Wide<A, B, C, D, E, F, G, H, I, J, K> {
    a: A,
    b: B,
    c: C,
    d: D,
    e: E,
    f: F,
    g: G,
    h: H,
    i: I,
    j: J,
    k: K,
}

#[test]
fn type_parameters_past_j_are_named() {
    let ints = Wide::<i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32>::elm_definition();
    let mixed = Wide::<u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, String>::elm_definition();
    assert_eq!(ints, mixed);
    let definition = ints.unwrap();
    assert!(
        definition.contains("type alias Wide a b c d e f g h i j t10"),
        "{definition}"
    );
    let encoder =
        Wide::<i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32>::encoder_definition()
            .unwrap();
    assert!(encoder.contains("t10Encoder"), "{encoder}");
}
