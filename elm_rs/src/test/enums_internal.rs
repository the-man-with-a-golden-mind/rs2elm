use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
#[serde(tag = "t")]
enum Enum {
    Unit1,
    Unit2,
    Named1 { field: i32 },
    Named2 { field: i32 },
}

#[test]
fn variants() {
    super::test_json_cases(
        [
            Enum::Unit1,
            Enum::Unit2,
            Enum::Named1 { field: 123 },
            Enum::Named2 { field: 234 },
        ]
        .into_iter()
        .map(|value| (serde_json::to_string(&value).unwrap(), value))
        .collect(),
        "",
    );
}
