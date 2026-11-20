#![no_implicit_prelude]
#![allow(dead_code)]

#[derive(crate::rs2elm::Elm, crate::rs2elm::ElmEncode, crate::rs2elm::ElmDecode)]
enum Filetype {
    Jpeg,
    Png,
}

#[derive(crate::rs2elm::Elm, crate::rs2elm::ElmEncode, crate::rs2elm::ElmDecode)]
struct Drawing {
    filename: ::std::string::String,
    filetype: Filetype,
}

#[derive(crate::rs2elm::Elm, crate::rs2elm::ElmQuery)]
struct Query {
    page: u8,
    name: ::std::string::String,
}

#[test]
fn hygiene() {
    let mut target = ::std::vec![];
    crate::rs2elm::export!("Bindings", &mut target, {
        encoders: [Drawing, Filetype],
        decoders: [Drawing, Filetype],
        queries: [Query],
    })
    .unwrap();
    let source = ::std::string::String::from_utf8(target).unwrap();
    assert!(source.contains("urlEncodeQuery"));
}
