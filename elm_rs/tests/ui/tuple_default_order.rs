use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Bad(#[serde(default)] Option<i32>, i32);

fn main() {}
