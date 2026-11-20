use rs2elm::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

mod codec {
    pub fn serialize<S: super::Serializer>(value: &u8, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(*value)
    }

    pub fn deserialize<'de, D: super::Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
        <u8 as super::Deserialize>::deserialize(deserializer)
    }
}

#[derive(Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct WithAttr {
    #[serde(with = "codec")]
    value: u8,
}

fn main() {}
