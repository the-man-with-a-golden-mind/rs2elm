# Changelog

## 0.1.0

First release of `rs2elm` and `rs2elm_derive`.

This is a maintained successor to [elm_rs](https://github.com/Heliozoa/elm_rs) 0.2.3 by Heliozoa. The supported serde attributes match `serde_json`, including `flatten` (also inside an enum struct variant), `alias`, `skip`, `skip_serializing`, `skip_deserializing`, `skip_serializing_if`, `deny_unknown_fields`, and `default` for `Option` and `()`.

Forms encode and decode `application/x-www-form-urlencoded` bodies. `#[rs2elm(index)]` numbers list elements. `socket_send` and `socket_listen` generate Elm 0.19 ports around one JSON text frame. Direct recursion is decoded with `Json.Decode.lazy`. Mutual recursion uses `#[rs2elm(lazy)]`. `#[rs2elm(rename)]` sets the Elm name.
