use rs2elm::{Elm, ElmQuery};

#[derive(Elm, ElmQuery)]
struct Inner<T> {
    value: T,
}

#[derive(Elm, ElmQuery)]
struct Outer<T> {
    child: Inner<Vec<T>>,
}

fn main() {}
