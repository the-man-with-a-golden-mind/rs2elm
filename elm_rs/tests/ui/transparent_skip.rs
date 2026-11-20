use rs2elm::{Elm, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm, ElmEncode)]
#[serde(transparent)]
struct Bad {
    #[serde(skip)]
    inner: u8,
}

fn main() {}
