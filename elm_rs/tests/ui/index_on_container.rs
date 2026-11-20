use rs2elm::Elm;

#[derive(Elm)]
#[rs2elm(index)]
struct Bad {
    a: i32,
}

fn main() {}
