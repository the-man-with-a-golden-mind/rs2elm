use rs2elm::ElmQuery;

struct Item<T> {
    n: T,
}

#[derive(ElmQuery)]
struct Bad<T> {
    #[rs2elm(index)]
    item: Item<T>,
}

fn main() {}
