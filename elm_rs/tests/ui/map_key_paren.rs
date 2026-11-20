#![allow(unused_parens)]

use rs2elm::Elm;

#[derive(Elm)]
struct Bad<T> {
    rows: std::collections::HashMap<(String), T>,
}

fn main() {}
