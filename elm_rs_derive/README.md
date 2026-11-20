# rs2elm_derive

Derive macros for [`rs2elm`](https://github.com/the-man-with-a-golden-mind/rs2elm): `Elm`, `ElmEncode`, `ElmDecode`, `ElmQuery`, and `ElmQueryField`.

This crate is heavily based on [elm_rs_derive](https://github.com/Heliozoa/elm_rs) by Heliozoa. The generated codecs are fixed here, and the crate is actively maintained.

Depend on `rs2elm` with its `derive` feature. That enables these macros. Depending on `rs2elm_derive` directly is for the macros alone.

## Features

- `json`: `ElmEncode` and `ElmDecode`.
- `serde`: serde attributes on those derives. A supported attribute changes the JSON codec. An unsupported attribute fails the build.
- `query`: `ElmQuery` and `ElmQueryField`.

`rs2elm` turns `json`, `query`, and `serde` on for this crate when you use the default `derive` and `serde` features.

## License

Licensed under Mozilla Public License Version 2.0.

rs2elm_derive is based on elm_rs_derive by Heliozoa, which is also MPL-2.0.
