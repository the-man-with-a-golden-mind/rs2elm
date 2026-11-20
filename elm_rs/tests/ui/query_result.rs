use rs2elm::{Elm, ElmQuery};

#[derive(Elm, ElmQuery)]
struct Form {
    outcome: Result<i32, String>,
}

fn main() {}
