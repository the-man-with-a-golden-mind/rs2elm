use rs2elm::{Elm, ElmQuery};
use std::collections::HashMap;

#[derive(Elm, ElmQuery)]
struct Form {
    extra: HashMap<String, i32>,
}

fn main() {}
