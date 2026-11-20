use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmDecode, ElmEncode)]
#[serde(tag = "t")]
enum Bad {
    T(i32, i32),
}

fn main() {}
