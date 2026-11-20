use rs2elm::{Elm, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode)]
struct Bad {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option,
}

fn main() {}
