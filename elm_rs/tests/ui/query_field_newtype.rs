use rs2elm::{Elm, ElmQueryField};

#[derive(Elm, ElmQueryField)]
enum Choice {
    One(u8),
}

fn main() {}
