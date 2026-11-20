use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum Recurse {
    Unit,
    Recursive(Box<Recurse>),
}

#[test]
fn enum_direct_recursion() {
    let definition = Recurse::elm_definition().unwrap();
    assert!(definition.contains("type Recurse"), "{definition}");
    let decoder = Recurse::decoder_definition().unwrap();
    assert!(decoder.contains("Json.Decode.lazy"), "{decoder}");
    super::test_json(Recurse::Recursive(Box::new(Recurse::Recursive(Box::new(
        Recurse::Unit,
    )))));
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

#[test]
fn struct_direct_recursion() {
    let definition = Node::elm_definition().unwrap();
    assert!(definition.starts_with("type Node\n"), "{definition}");
    assert!(definition.contains("next : Maybe (Node)"), "{definition}");
    assert!(!definition.contains("type alias"), "{definition}");
    super::test_json(Node {
        value: 1,
        next: Some(Box::new(Node {
            value: 2,
            next: None,
        })),
    });
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct MutualA {
    n: u8,
    #[rs2elm(lazy)]
    other: Option<Box<MutualB>>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct MutualB {
    n: u8,
    #[rs2elm(lazy)]
    other: Option<Box<MutualA>>,
}

#[test]
fn mutual_recursion_uses_lazy_attribute() {
    let source = format!(
        "module Mutual exposing (..)\n\nimport Json.Decode\nimport Json.Encode\n\n{}\n{}\n{}\n{}\n{}\n{}\n",
        MutualA::elm_definition().unwrap(),
        MutualB::elm_definition().unwrap(),
        MutualA::encoder_definition().unwrap(),
        MutualB::encoder_definition().unwrap(),
        MutualA::decoder_definition().unwrap(),
        MutualB::decoder_definition().unwrap(),
    );
    assert!(source.contains("Json.Decode.lazy"), "{source}");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../elm-test");
    let dir = std::env::temp_dir().join(format!("rs2elm_mutual_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::copy(root.join("elm.json"), dir.join("elm.json")).unwrap();
    fs::write(dir.join("src/Mutual.elm"), &source).unwrap();
    let output = Command::new("elm")
        .args(["make", "src/Mutual.elm", "--output=/dev/null"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let _ = fs::remove_dir_all(&dir);
    let json = serde_json::to_string(&MutualA {
        n: 1,
        other: Some(Box::new(MutualB {
            n: 2,
            other: Some(Box::new(MutualA { n: 3, other: None })),
        })),
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"n":1,"other":{"n":2,"other":{"n":3,"other":null}}}"#
    );
}

#[derive(Elm, ElmEncode, ElmDecode)]
#[allow(dead_code)]
struct CycleA {
    other: Option<Box<CycleB>>,
}

#[derive(Elm, ElmEncode, ElmDecode)]
#[allow(dead_code)]
struct CycleB {
    other: Option<Box<CycleA>>,
}

#[test]
fn mutual_recursion_without_lazy_fails_elm_make() {
    let source = format!(
        "module Cycle exposing (..)\n\nimport Json.Decode\nimport Json.Encode\n\n{}\n{}\n",
        CycleA::elm_definition().unwrap(),
        CycleB::elm_definition().unwrap(),
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../elm-test");
    let dir = std::env::temp_dir().join(format!("rs2elm_cycle_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::copy(root.join("elm.json"), dir.join("elm.json")).unwrap();
    fs::write(dir.join("src/Cycle.elm"), &source).unwrap();
    let output = Command::new("elm")
        .args(["make", "src/Cycle.elm", "--output=/dev/null"])
        .current_dir(&dir)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "elm make accepted a mutual cycle\n{stderr}\n{source}"
    );
    assert!(
        stderr.to_lowercase().contains("cycle") || stderr.contains("CycleA"),
        "{stderr}"
    );
    let _ = fs::remove_dir_all(&dir);
}
