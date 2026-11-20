use rs2elm::{Elm, ElmQuery};

#[derive(Elm, ElmQuery)]
struct Page {
    #[rs2elm(index)]
    page: i32,
}

fn main() {}
