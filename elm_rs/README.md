# rs2elm

rs2elm generates Elm 0.19 code from Rust types: the type, a JSON encoder and decoder, a form-urlencoded query codec, and WebSocket ports.

It is heavily based on [elm_rs](https://github.com/Heliozoa/elm_rs) by Heliozoa. The serde and Elm behavior from that project is fixed here, and this library is actively maintained.

You depend on `rs2elm`. It re-exports the derive macros. `rs2elm_derive` is the proc-macro crate.

```toml
[dependencies]
rs2elm = "0.1"
```

`derive` and `serde` are on by default. Optional features are `chrono`, `time`, and `uuid`.

The generated module imports `elm/json` when it encodes or decodes JSON, and `elm/url` when it builds a query. Install those Elm packages before compiling the file.

## Example

```rust
use rs2elm::{Elm, ElmDecode, ElmEncode, ElmQuery, ElmQueryField};

#[derive(Elm, ElmEncode, ElmDecode)]
enum Filetype {
    Jpeg,
    Png,
}

#[derive(Elm, ElmEncode, ElmDecode)]
struct Drawing {
    title: String,
    authors: Vec<String>,
    filename: String,
    filetype: Filetype,
}

#[derive(Elm, ElmQuery)]
struct Query {
    page: usize,
    thumbnail_size: Size,
}

#[derive(Elm, ElmQueryField)]
enum Size {
    Small,
    Large,
}

fn main() {
    let mut target = Vec::new();
    rs2elm::export!("Bindings", &mut target, {
        encoders: [Filetype, Drawing],
        decoders: [Filetype, Drawing],
        queries: [Query],
        query_fields: [Size],
    })
    .unwrap();
}
```

`export!` writes one Elm module. It also writes every type those items use, so exporting `Drawing` includes `Filetype`.

```elm
type Filetype
    = Jpeg
    | Png

type alias Drawing =
    { title : String
    , authors : List String
    , filename : String
    , filetype : Filetype
    }

drawingEncoder : Drawing -> Json.Encode.Value
drawingDecoder : Json.Decode.Decoder Drawing

type alias Query =
    { page : Int
    , thumbnailSize : Size
    }

urlEncodeQuery : Query -> List Url.Builder.QueryParameter
urlDecodeQuery : List (String, String) -> Result String Query
```

Elm field names are camelCase. JSON object keys and query keys use the serde name, which is the Rust field name unless you rename it.

## JSON

Structs become Elm records and JSON objects. A tuple struct becomes a JSON array. A newtype struct uses the inner JSON value. Externally tagged enums are the default: a unit variant is a string, and a struct variant is `{"Name": { ... }}`.

`#[serde(tag = "t")]` writes an internally tagged object. `#[serde(tag = "t", content = "c")]` writes an adjacent tag and payload. `#[serde(untagged)]` tries variants in order. `#[serde(transparent)]` uses the single field's JSON.

A direct cycle is one Elm type and is decoded with `Json.Decode.lazy`. When two types refer to each other, mark a field with `#[rs2elm(lazy)]`.

A generic type is one Elm type. `Foo<u32>` and `Foo<String>` share it. The codec takes the parameter's encoder or decoder:

```elm
fooEncoder : (a -> Json.Encode.Value) -> Foo a -> Json.Encode.Value
fooDecoder : Json.Decode.Decoder a -> Json.Decode.Decoder (Foo a)
```

`#[rs2elm(rename = "Name")]` sets the Elm type name. `#[serde(rename = "...")]` on a container does not change the Elm name or the JSON name.

### Serde attributes

Container: `rename_all`, `rename_all_fields`, `tag`, `content`, `untagged`, `transparent`, `deny_unknown_fields`, `default`.

Variant: `rename`, `rename_all`, `alias`, `skip`, `other`.

Field: `rename`, `alias`, `skip`, `skip_serializing`, `skip_deserializing`, `skip_serializing_if`, `default`, `flatten`.

`rename_all` renames struct fields, or enum variants. `rename_all_fields` renames the fields of struct variants. A variant's own `rename_all` wins for that variant.

`alias` accepts extra JSON names on decode. A field and its alias in the same object fail with `` duplicate field `name` ``.

`skip` omits the field or variant on both sides. `skip_serializing` omits it from the encoder and the decoder still requires it. `skip_deserializing` keeps it in the encoder and fills it on decode. On a tuple, `skip_deserializing` does not consume the next JSON element. On a newtype variant, `skip_deserializing` decodes the variant as a unit.

`skip_serializing_if` accepts `Option::is_none`, `Vec::is_empty`, and `String::is_empty`.

`default` works for `Option` and `()`. A missing or null `Option` is `Nothing`. A present value of the wrong type is an error. On a tuple, `default` is valid only when every later field also has `default`. Any other type fails the build, because Elm has no place to put a Rust `Default` value.

`flatten` merges a struct, a map, a newtype, or a transparent struct into the parent object. This includes a struct variant of an enum. A normal field keeps a shared name. Flattened structs take keys in source order. A flattened map leaves every key in place. `deny_unknown_fields` reports a key that is still left. An internal tag is not part of the flattened value.

`deny_unknown_fields` fails the decode on an unknown key. `other` is the fallback variant and must be last.

An internally tagged newtype variant or tuple variant fails the build. serde_json does not support those shapes.

`with`, `serialize_with`, `deserialize_with`, `from`, `into`, `try_from`, and any other `skip_serializing_if` path fail the build.

Elm reads JSON with `JSON.parse`. A repeated key such as `{"a": 1, "a": 2}` keeps one value. The alias rule above is the duplicate Elm can see.

### Integers

Every integer is an Elm `Int` and a JSON number, including `u64`, `i64`, `u128`, `i128`, `usize`, `isize`, and the matching `NonZero` and atomic types. Elm is exact through `2^53 - 1` (`9007199254740991`). `u32` and every narrower integer are inside that range. A wider value is rounded by JavaScript to the nearest number it can store. It is not clamped to `2^53 - 1`.

### Built-in types

`chrono` and `uuid` values are Elm `String`s, matching `serde_json`.

With the `time` feature, `Date`, `Time`, `PrimitiveDateTime`, and `OffsetDateTime` are Elm records. `time::Duration` is the record `TimeDuration`. Encoders write the integer arrays that `serde_json` produces when `time` has `serde` and does not have `serde-human-readable`.

`std::time::Duration` is `{ secs : Int, nanos : Int }`. `SystemTime` is `{ secs_since_epoch : Int, nanos_since_epoch : Int }`. These two records are not `serde_json` values.

`IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr`, `SocketAddrV4`, and `SocketAddrV6` are strings. `PhantomData` is JSON `null` and is omitted from the Elm type's fields.

## Forms

`urlEncodeQuery` returns `List Url.Builder.QueryParameter`. That list is the body of an `application/x-www-form-urlencoded` form. `urlDecodeQuery` reads `List (String, String)`.

`Option` omits the key when the value is `Nothing`. `Vec`, slices, and arrays repeat one key. A nested query writes bracket keys such as `thumbnail[width]`. `Url.Builder.toQuery` percent-encodes `[` and `]` as `%5B` and `%5D`. The decoder reads the decoded names, `thumbnail[width]`, not the percent-encoded text.

`bool` and floats are strings (`"true"`, `"false"`, `String.fromFloat`). A missing field is `Err ("missing " ++ key)`. A bad integer is `Err "invalid int"`.

`#[rs2elm(index)]` on a list numbers each element (`shots[0][width]`). An indexed `Vec<Option<_>>` keeps a `Nothing` in the middle and omits a trailing `Nothing`. A gap in a required indexed list is `Err ("missing " ++ key)`.

## WebSockets

`socket_send` and `socket_listen` in `export!` make the file a `port module`. Each value is one JSON text frame. JavaScript owns the socket.

```elm
port clientMsgOutgoing : String -> Cmd msg

sendClientMsg : ClientMsg -> Cmd msg
sendClientMsg message =
    clientMsgOutgoing (Json.Encode.encode 0 (clientMsgEncoder message))

port serverMsgIncoming : (String -> msg) -> Sub msg

listenServerMsg : (Result Json.Decode.Error ServerMsg -> msg) -> Sub msg
listenServerMsg tagger =
    serverMsgIncoming (\raw -> tagger (Json.Decode.decodeString serverMsgDecoder raw))
```

## Names

Elm constructors live in one namespace. `export!` fails the build when two exported types share a constructor, including a constructor that arrives through a dependency.

```rust,ignore
enum Enum1 {
    Variant,
}
enum Enum2 {
    Variant,
}
```

## License

Licensed under Mozilla Public License Version 2.0.

rs2elm is based on elm_rs by Heliozoa, which is also MPL-2.0.
