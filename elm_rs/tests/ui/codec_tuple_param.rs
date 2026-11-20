use rs2elm::{Elm, ElmEncode};

#[derive(Elm, ElmEncode)]
struct Bad<T> {
    pair: (T, i32),
}

fn main() {}
