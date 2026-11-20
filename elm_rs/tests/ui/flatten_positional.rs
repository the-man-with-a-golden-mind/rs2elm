use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Inner {
    a: u8,
}

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct FlatNewtype(#[serde(flatten)] Inner);

fn main() {}
