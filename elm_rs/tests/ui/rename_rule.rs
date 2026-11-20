use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(rename_all = "TitleCase")]
struct Named {
    some_field: u8,
}

fn main() {}
