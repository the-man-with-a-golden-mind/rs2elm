use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Named {
    #[serde(flatten)]
    pair: (u8, u8),
}

fn main() {}
