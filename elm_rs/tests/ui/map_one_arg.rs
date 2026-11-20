use rs2elm::Elm;

#[derive(Elm)]
struct Bad<T> {
    rows: std::collections::HashMap<T>,
}

fn main() {}
