use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Named {
    #[serde(skip_serializing_if = "String::is_empty")]
    value: i32,
}

fn main() {}
