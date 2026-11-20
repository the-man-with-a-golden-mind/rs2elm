use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Named {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    value: String,
}

fn main() {}
