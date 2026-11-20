//! Codecs for the `time` crate.
//!
//! With `time`'s `serde` feature and without `serde-human-readable`, `serde_json`
//! writes each value as an array of integers. The Elm value is a record so the
//! parts have names. The encoder lists those fields in serde's order.
//!
//! `time::Duration` is `TimeDuration` in Elm. `std::time::Duration` already owns
//! the Elm name `Duration`.

use crate::{Elm, ElmDecode, ElmEncode};

fn camel_name(elm_type: &str, suffix: &str) -> String {
    let (first, rest) = elm_type.split_at(1);
    format!("{}{rest}{suffix}", first.to_ascii_lowercase())
}

fn definition(elm_type: &str, fields: &[&str]) -> String {
    let mut out = format!("type alias {elm_type} =\n");
    for (index, field) in fields.iter().enumerate() {
        if index == 0 {
            out.push_str(&format!("    {{ {field} : Int\n"));
        } else {
            out.push_str(&format!("    , {field} : Int\n"));
        }
    }
    out.push_str("    }\n");
    out
}

fn encoder(elm_type: &str, fields: &[&str]) -> String {
    let name = camel_name(elm_type, "Encoder");
    let mut out = format!(
        "{name} : {elm_type} -> Json.Encode.Value\n{name} value =\n    Json.Encode.list identity\n    ["
    );
    for (index, field) in fields.iter().enumerate() {
        if index == 0 {
            out.push_str(&format!(" Json.Encode.int value.{field}\n"));
        } else {
            out.push_str(&format!("    , Json.Encode.int value.{field}\n"));
        }
    }
    out.push_str("    ]\n");
    out
}

fn decoder(elm_type: &str, fields: &[&str]) -> String {
    let name = camel_name(elm_type, "Decoder");
    let mut out = format!(
        "{name} : Json.Decode.Decoder {elm_type}\n{name} =\n    Json.Decode.succeed {elm_type}\n"
    );
    for index in 0..fields.len() {
        out.push_str(&format!(
            "    |> Json.Decode.andThen (\\x -> Json.Decode.map x (Json.Decode.index {index} Json.Decode.int))\n"
        ));
    }
    out
}

macro_rules! impl_time {
    ($ty:ty, $elm:literal, $($field:literal),+ $(,)?) => {
        impl Elm for $ty {
            fn elm_type() -> String {
                $elm.to_string()
            }

            fn elm_definition() -> Option<String> {
                Some(definition($elm, &[$($field),+]))
            }
        }

        impl ElmEncode for $ty {
            fn encoder_type() -> String {
                camel_name($elm, "Encoder")
            }

            fn encoder_definition() -> Option<String> {
                Some(encoder($elm, &[$($field),+]))
            }
        }

        impl ElmDecode for $ty {
            fn decoder_type() -> String {
                camel_name($elm, "Decoder")
            }

            fn decoder_definition() -> Option<String> {
                Some(decoder($elm, &[$($field),+]))
            }
        }
    };
}

impl_time!(time::Date, "Date", "year", "ordinal");
impl_time!(time::Time, "Time", "hour", "minute", "second", "nanosecond");
impl_time!(
    time::PrimitiveDateTime,
    "PrimitiveDateTime",
    "year",
    "ordinal",
    "hour",
    "minute",
    "second",
    "nanosecond"
);
impl_time!(
    time::OffsetDateTime,
    "OffsetDateTime",
    "year",
    "ordinal",
    "hour",
    "minute",
    "second",
    "nanosecond",
    "offsetHour",
    "offsetMinute",
    "offsetSecond"
);
impl_time!(time::Duration, "TimeDuration", "seconds", "nanoseconds");
