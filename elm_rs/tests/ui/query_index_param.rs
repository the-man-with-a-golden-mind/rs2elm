use rs2elm::ElmQuery;

#[derive(ElmQuery)]
struct Bad<T> {
    #[rs2elm(index)]
    item: T,
}

fn main() {}
