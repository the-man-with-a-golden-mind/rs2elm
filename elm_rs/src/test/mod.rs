use crate::{Elm, ElmDecode, ElmEncode, ElmQuery, ElmQueryField};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fmt::Debug,
    io::Write,
    process::{Command, Stdio},
};

mod complex;
mod coverage;
mod enums_adjacent;
mod enums_external;
mod enums_internal;
mod enums_untagged;
mod etc_serde;
mod export_module;
mod hygiene;
mod nested;
mod query;
mod recursive;
mod regression;
mod socket;
mod structs;
mod structs_serde;
mod types;

fn test_json<T: Elm + ElmEncode + ElmDecode + Serialize + DeserializeOwned + PartialEq + Debug>(
    t: T,
) {
    test_json_cases(vec![(serde_json::to_string(&t).unwrap(), t)], "");
}

fn test_json_with_deps<
    T: Elm + ElmEncode + ElmDecode + Serialize + DeserializeOwned + PartialEq + Debug,
>(
    t: T,
    deps: &str,
) {
    test_json_cases(vec![(serde_json::to_string(&t).unwrap(), t)], deps);
}

/// One Elm process decodes every JSON text and encodes it again.
fn test_json_cases<
    T: Elm + ElmEncode + ElmDecode + Serialize + DeserializeOwned + PartialEq + Debug,
>(
    cases: Vec<(String, T)>,
    deps: &str,
) {
    let jsons: Vec<&str> = cases.iter().map(|(json, _)| json.as_str()).collect();
    let decoded: Vec<T> = round_trip_json(&jsons, deps);
    let expected: Vec<T> = cases.into_iter().map(|(_, value)| value).collect();
    assert_eq!(decoded, expected);
}

fn test_json_without_eq<
    T: Elm + ElmEncode + ElmDecode + ElmDecode + Serialize + DeserializeOwned + Debug,
>(
    t: &T,
    deps: &str,
) -> T {
    let json = serde_json::to_string(t).unwrap();
    round_trip_json(&[json.as_str()], deps).remove(0)
}

fn round_trip_json<T: Elm + ElmEncode + ElmDecode + DeserializeOwned + Debug>(
    jsons: &[&str],
    deps: &str,
) -> Vec<T> {
    let samples = jsons
        .iter()
        .map(|json| format!("\"{}\"", json.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let encoder_type = T::encoder_type();
    let decoder_type = T::decoder_type();
    let elm_type = T::elm_definition().unwrap();
    let encoder = T::encoder_definition().unwrap();
    let decoder = T::decoder_definition().unwrap();
    let codec = format!("{deps}\n{elm_type}\n{encoder}\n{decoder}");
    let helpers = result_helpers(&codec);
    let pairs = pairs_map_helper(&codec);
    let dict = if codec.contains("Dict") || helpers.contains("Dict") {
        "import Dict exposing (Dict)\n"
    } else {
        ""
    };

    let input = format!(
        r#"
import Json.Decode
import Json.Encode
{dict}
{pairs}
{helpers}
{deps}

{elm_type}

{encoder}

{decoder}

encodeSample json =
    case Json.Decode.decodeString {decoder_type} json of
        Ok value ->
            {encoder_type} value
        Err err ->
            Json.Encode.string (Json.Decode.errorToString err)

reencoded =
    Json.Encode.list identity (List.map encodeSample [ {samples} ])

s =
    Json.Encode.encode 0 reencoded

"START"
s
"END"

:exit
"#
    );

    let json = run_repl(&input);
    let unescaped = unescape::unescape(&json).unwrap_or(json);
    serde_json::from_str(&unescaped).unwrap_or_else(|err| panic!("{err}\n{unescaped}"))
}

fn pairs_map_helper(source: &str) -> &'static str {
    if source.contains("elmRsPairsMap") && !source.contains("elmRsPairsMap decoder") {
        crate::elm_decode::OBJECT_PAIRS_MAP
    } else {
        ""
    }
}

fn result_helpers(source: &str) -> String {
    let mut parts = Vec::new();
    if source.contains("resultDecoder") {
        parts.push(Result::<(), ()>::decoder_definition().unwrap());
    }
    if source.contains("resultEncoder") {
        parts.push(Result::<(), ()>::encoder_definition().unwrap());
    }
    if parts.is_empty() {
        return String::new();
    }
    // elm repl 0.19.1 crashes (Map.!) when two top-level declarations are not
    // separated by a blank line.
    let mut helpers = parts.join("\n\n");
    helpers.push('\n');
    helpers
}

fn elm_text<T: Elm + ElmEncode + ElmDecode>(body: &str) -> String {
    elm_text_with_deps::<T>("", body)
}

fn elm_text_with_deps<T: Elm + ElmEncode + ElmDecode>(deps: &str, body: &str) -> String {
    let elm = T::elm_definition().unwrap();
    let encoder = T::encoder_definition().unwrap();
    let decoder = T::decoder_definition().unwrap();
    let source = format!("{deps}\n{elm}\n{encoder}\n{decoder}");
    let pairs = pairs_map_helper(&source);
    let dict = if source.contains("Dict") {
        "import Dict exposing (Dict)\n"
    } else {
        ""
    };
    let input = format!(
        r#"
import Json.Decode
import Json.Encode
{dict}
{pairs}
{deps}

{elm}

{encoder}

{decoder}

s =
{body}

"START"
s
"END"

:exit
"#
    );
    let raw = run_repl(&input);
    unescape::unescape(&raw).unwrap_or(raw)
}

fn test_query<
    T: Elm + ElmEncode + ElmDecode + ElmQuery + Serialize,
    U: Elm + ElmEncode + ElmDecode + ElmQueryField + Serialize,
>(
    val: T,
    expected: &str,
) {
    test_query_with_deps::<T, U>(val, expected, "");
}

fn test_query_with_deps<
    T: Elm + ElmEncode + ElmDecode + ElmQuery + Serialize,
    U: Elm + ElmEncode + ElmDecode + ElmQueryField + Serialize,
>(
    val: T,
    expected: &str,
    deps: &str,
) {
    let query_function = format!("urlEncode{}", T::elm_type());
    test_query_call::<T, U>(val, expected, deps, &format!("{query_function} value"));
}

/// `call` is the Elm expression applied to the decoded `value`.
/// A generic query passes its encoders there, for example `urlEncodeWrap String.fromInt value`.
fn test_query_call<
    T: Elm + ElmEncode + ElmDecode + ElmQuery + Serialize,
    U: Elm + ElmEncode + ElmDecode + ElmQueryField + Serialize,
>(
    val: T,
    expected: &str,
    deps: &str,
    call: &str,
) {
    let json = serde_json::to_string(&val).unwrap().replace("\"", "\\\"");

    let decoder_type = T::decoder_type();
    let elm_type = T::elm_definition().unwrap();
    let query = T::elm_query();
    let decoder = T::decoder_definition().unwrap();

    let u_elm_type = U::elm_definition().unwrap();
    let u_decoder = U::decoder_definition().unwrap();
    let u_query = U::query_field_encoder_definition().unwrap();
    let pairs = pairs_map_helper(&format!("{deps}\n{decoder}\n{u_decoder}"));
    let encode_import = if pairs.is_empty() {
        ""
    } else {
        "import Json.Encode\n"
    };

    let input = format!(
        r#"
import Json.Decode
{encode_import}import Url.Builder

{pairs}
{deps}

{u_elm_type}

{elm_type}

{u_decoder}

{decoder}

{u_query}

{query}

decoded = Json.Decode.decodeString {decoder_type} "{json}"


s = case decoded of
    Ok value ->
        Url.Builder.toQuery ({call})
    Err err ->
        Json.Decode.errorToString err

"START"
s
"END"

:exit
"#,
    );
    let output = run_repl(&input);
    assert_eq!(output, expected);
}

/// One Elm process decodes every form-pair list for `T`.
/// `call` is `urlDecodeFoo` or `urlDecodeFoo elmRsFormValueInt` for a generic.
fn test_form_cases<T>(cases: Vec<(Vec<(&str, &str)>, Result<T, &str>)>, deps: &str, call: &str)
where
    T: Elm + ElmEncode + ElmDecode + ElmQuery + Serialize + DeserializeOwned + PartialEq + Debug,
{
    let samples = cases
        .iter()
        .map(|(pairs, _)| {
            let body = pairs
                .iter()
                .map(|(key, value)| {
                    format!("( \"{}\", \"{}\" )", escape_elm(key), escape_elm(value))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("[ {body} ]")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let input = format!(
        r#"
import Json.Decode
import Json.Encode

{helpers}

{deps}

{elm}

{encoder}

{form}

decodeSample pairs =
    case {call} pairs of
        Ok value ->
            "ok:" ++ Json.Encode.encode 0 ({encoder_type} value)
        Err err ->
            "err:" ++ err

reencoded =
    Json.Encode.list Json.Encode.string (List.map decodeSample [ {samples} ])

s =
    Json.Encode.encode 0 reencoded

"START"
s
"END"

:exit
"#,
        helpers = crate::form_helpers(),
        elm = T::elm_definition().unwrap(),
        encoder = T::encoder_definition().unwrap(),
        encoder_type = T::encoder_type(),
        form = T::elm_form(),
    );
    let raw = run_repl(&input);
    let unescaped = unescape::unescape(&raw).unwrap_or(raw);
    let outputs: Vec<String> =
        serde_json::from_str(&unescaped).unwrap_or_else(|err| panic!("{err}\n{unescaped}"));
    assert_eq!(outputs.len(), cases.len());
    for (output, (_, expected)) in outputs.iter().zip(cases) {
        match expected {
            Ok(value) => {
                assert!(output.starts_with("ok:"), "{output}");
                let actual: T = serde_json::from_str(&output[3..])
                    .unwrap_or_else(|err| panic!("{err}\n{output}"));
                assert_eq!(actual, value);
            }
            Err(message) => assert_eq!(output, &format!("err:{message}")),
        }
    }
}

fn escape_elm(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_repl(input: &str) -> String {
    let mut cmd = Command::new("elm")
        .arg("repl")
        .current_dir("../elm-test")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = cmd.stdin.take().unwrap();
    stdin.write_all(input.as_bytes()).unwrap();

    let output = cmd.wait_with_output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    if !output.status.success() {
        panic!("elm repl failed\n{stderr}\n{stdout}");
    }

    let mut reading = false;
    for line in stdout.lines() {
        if line.contains("END") {
            break;
        }
        if reading {
            let first_quote = line.find('"').unwrap();
            let last_quote = line.rfind('"').unwrap();
            return line[first_quote + 1..last_quote].to_string();
        }
        if line.contains("START") {
            reading = true;
        }
    }
    panic!("elm repl produced no value\n{stderr}\n{stdout}");
}

#[cfg(any(not(feature = "derive"), not(feature = "serde")))]
compile_error!("The tests require the `derive` and `serde` features to be activated.");
