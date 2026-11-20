use rs2elm::Elm;
use std::collections::HashMap;

#[derive(Elm)]
struct Rows<T> {
    rows: HashMap<T, i32>,
}

fn main() {}
