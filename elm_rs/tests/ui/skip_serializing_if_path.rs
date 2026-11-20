use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

fn skip(_: &u8) -> bool {
    false
}

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct SkipPath {
    #[serde(skip_serializing_if = "skip")]
    value: u8,
}

fn main() {}
