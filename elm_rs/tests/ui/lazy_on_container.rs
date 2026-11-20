use rs2elm::Elm;

#[derive(Elm)]
#[rs2elm(lazy)]
struct Bad {
    a: i32,
}

fn main() {}
