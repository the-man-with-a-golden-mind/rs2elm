#![allow(dead_code)]

use crate::{Elm, ElmDecode, ElmEncode, ElmQuery, ElmQueryField};
use std::{fs, path::PathBuf, process::Command};

#[allow(dead_code)]
#[derive(Elm, ElmEncode, ElmDecode)]
enum Filetype {
    Jpeg,
    Png,
}

#[derive(Elm, ElmEncode, ElmDecode)]
struct Drawing {
    title: String,
    authors: Vec<String>,
    filename: String,
    filetype: Filetype,
}

#[derive(Elm, ElmQuery)]
struct Query {
    page: usize,
    thumbnail_size: Size,
}

#[derive(Elm, ElmQueryField)]
enum Size {
    Small,
    Large,
}

#[test]
fn readme_export_compiles_with_elm() {
    let mut target = Vec::new();
    crate::export!("Bindings", &mut target, {
        encoders: [Drawing],
        decoders: [Drawing],
        queries: [Query],
    })
    .unwrap();
    let source = String::from_utf8(target).unwrap();

    assert!(!source.contains("import Http"));
    assert!(!source.contains("import Dict"));
    assert!(source.contains("import Json.Decode"));
    assert!(source.contains("import Json.Encode"));
    assert!(source.contains("import Url.Builder"));
    assert!(!source.contains("resultEncoder"));
    assert!(!source.contains("resultDecoder"));

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../elm-test");
    let dir = std::env::temp_dir().join(format!("rs2elm_export_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::copy(root.join("elm.json"), dir.join("elm.json")).unwrap();
    fs::write(dir.join("src/Bindings.elm"), &source).unwrap();
    fs::write(
        dir.join("src/Main.elm"),
        r#"
module Main exposing (main)

import Bindings exposing (Size(..))
import Platform

main : Program () () ()
main =
    Platform.worker
        { init =
            \_ ->
                let
                    _ =
                        Bindings.urlEncodeQuery
                            { page = 1
                            , thumbnailSize = Small
                            }
                in
                ( (), Cmd.none )
        , update = \_ model -> ( model, Cmd.none )
        , subscriptions = \_ -> Sub.none
        }
"#,
    )
    .unwrap();

    let output = Command::new("elm")
        .arg("make")
        .arg("src/Main.elm")
        .arg("--output=/dev/null")
        .current_dir(&dir)
        .output()
        .expect("elm is installed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "elm make failed\nstdout:\n{stdout}\nstderr:\n{stderr}\nsource:\n{source}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[derive(Elm, ElmEncode)]
enum ClashOne {
    Variant,
}

#[derive(Elm, ElmEncode)]
enum ClashTwo {
    Variant,
}

#[test]
fn shared_constructor_is_rejected() {
    let mut target = Vec::new();
    let message = crate::export!("Bindings", &mut target, {
        encoders: [ClashOne, ClashTwo],
    })
    .expect_err("shared constructors must fail")
    .to_string();
    assert!(message.contains("Variant"), "{message}");
}

#[derive(Elm, ElmEncode)]
enum DepClashInner {
    Variant,
}

#[derive(Elm, ElmEncode)]
struct DepClashOuter {
    inner: DepClashInner,
}

#[derive(Elm, ElmEncode)]
enum DepClashOther {
    Variant,
}

#[test]
fn shared_constructor_in_a_dependency_is_rejected() {
    let mut target = Vec::new();
    let message = crate::export!("Bindings", &mut target, {
        encoders: [DepClashOuter, DepClashOther],
    })
    .expect_err("a dependency constructor must fail")
    .to_string();
    assert!(message.contains("Variant"), "{message}");
    assert!(message.contains("DepClashInner"), "{message}");
    assert!(message.contains("DepClashOther"), "{message}");
}

#[derive(Elm, ElmEncode, ElmDecode)]
struct Slot<T> {
    value: T,
}

#[derive(Elm, ElmEncode, ElmDecode)]
struct Page<T> {
    row: T,
    extra: std::collections::HashMap<String, i32>,
    outcome: Result<T, String>,
    slot: Slot<T>,
}

#[derive(Elm, ElmQueryField)]
enum Tone {
    Quiet,
    Loud,
}

#[test]
fn export_generic_module_collects_dependencies() {
    let mut target = Vec::new();
    crate::export!("Bindings", &mut target, {
        encoders: [Page<i32>],
        decoders: [Page<i32>],
        query_fields: [Tone],
    })
    .unwrap();
    let source = String::from_utf8(target).unwrap();
    assert!(source.contains("import Dict"), "{source}");
    assert!(source.contains("resultEncoder"), "{source}");
    assert!(source.contains("resultDecoder"), "{source}");
    assert!(source.contains("type alias Page a"), "{source}");
    assert!(source.contains("type alias Slot a"), "{source}");
    assert!(
        source.contains("(a -> Json.Encode.Value) -> Page a -> Json.Encode.Value"),
        "{source}"
    );
    assert!(
        source.contains("(Json.Decode.Decoder a) -> Json.Decode.Decoder (Page a)"),
        "{source}"
    );
    assert!(source.contains("queryFieldEncoderTone"), "{source}");
}
