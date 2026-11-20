use rs2elm::ElmQueryField;

#[derive(ElmQueryField)]
#[rs2elm(nope)]
enum Bad {
    A,
}

fn main() {}
