use rs2elm::Elm;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Elm)]
struct Only(#[serde(skip)] u8);

fn main() {}
