use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct FlatString {
    #[serde(flatten)]
    s: String,
}

fn main() {}
