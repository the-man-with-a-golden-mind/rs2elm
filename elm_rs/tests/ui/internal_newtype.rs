use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmDecode, ElmEncode)]
#[serde(tag = "t")]
enum Bad {
    N(i32),
}

fn main() {}
