use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Named {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: i32,
}

fn main() {}
