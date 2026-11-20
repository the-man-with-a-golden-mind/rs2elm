use rs2elm::Elm;

#[derive(Elm)]
enum Choice {
    One,
    #[rs2elm(rename = "One")]
    Two,
}

fn main() {}
