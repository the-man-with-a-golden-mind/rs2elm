use rs2elm::{Elm, ElmQuery};
use std::marker::PhantomData;

#[derive(Elm, ElmQuery)]
struct Form {
    ghost: PhantomData<u8>,
}

fn main() {}
