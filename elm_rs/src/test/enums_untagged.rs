use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(untagged)]
enum Enum {
    Unit1,
    Unit2,
    Newtype1(i32),
    Newtype2(i32),
    Tuple1(i32, i32),
    Tuple2(i32, i32),
    Named1 { field: i32 },
    Named2 { field: i32 },
}

#[test]
fn variants() {
    super::test_json_cases(
        [
            Enum::Unit1,
            Enum::Newtype1(123),
            Enum::Tuple1(123, 234),
            Enum::Named1 { field: 123 },
        ]
        .into_iter()
        .map(|value| (serde_json::to_string(&value).unwrap(), value))
        .collect(),
        "",
    );
}
