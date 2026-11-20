use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Elm, ElmDecode, ElmEncode, Clone, Eq, PartialEq)]
struct NestedTypes {
    result: Result<Vec<String>, u32>,
    option: Option<Vec<String>>,
    vec: Vec<Vec<String>>,
}

#[test]
fn nestedtypes() {
    super::test_json(NestedTypes {
        result: Ok(vec![]),
        option: Some(vec![]),
        vec: vec![vec![]],
    });
}
