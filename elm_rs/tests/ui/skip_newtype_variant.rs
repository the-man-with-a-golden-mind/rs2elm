use rs2elm::{Elm, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode)]
enum Bad {
    V(#[serde(skip)] u8),
}

fn main() {}
