use rs2elm::{Elm, ElmDecode, ElmEncode, ElmQuery};

#[derive(Elm, ElmEncode, ElmDecode, ElmQuery)]
#[rs2elm(nope)]
struct Bad {
    a: i32,
}

fn main() {}
