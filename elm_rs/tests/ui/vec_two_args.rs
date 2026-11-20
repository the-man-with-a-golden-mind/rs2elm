use rs2elm::Elm;

#[derive(Elm)]
struct Bad<T> {
    items: Vec<T, std::alloc::Global>,
}

fn main() {}
