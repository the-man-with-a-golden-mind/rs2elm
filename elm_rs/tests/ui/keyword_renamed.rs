use rs2elm::Elm;

#[derive(Elm)]
struct Named {
    #[rs2elm(rename = "type")]
    value: u8,
}

fn main() {}
