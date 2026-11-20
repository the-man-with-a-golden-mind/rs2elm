use rs2elm::{Elm, ElmEncode, ElmQuery};

struct Bucket<T, const N: usize> {
    value: T,
}

#[derive(Elm, ElmEncode, ElmQuery)]
struct Bad<T> {
    bucket: Bucket<T, 2>,
}

fn main() {}
