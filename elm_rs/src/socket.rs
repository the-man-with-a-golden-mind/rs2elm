//! Elm 0.19 ports for JSON text frames.
//!
//! `send` writes one JSON string to an outgoing port. `listen` reads one JSON
//! string from an incoming port. The page's JavaScript owns the WebSocket.

use crate::{Elm, ElmDecode, ElmEncode, export::Module, instrument};

/// Register `send{Type}` and the outgoing port. The payload is one JSON text frame.
#[instrument(skip(module), fields(elm = T::elm_type()))]
pub fn contribute_send<T: Elm + ElmEncode>(module: &mut Module) {
    T::contribute_encoder(module);
    let elm = T::elm_type();
    let name = format!("send{elm}");
    if !module.mark(&name) {
        return;
    }
    module.push_definition(&send_source::<T>(&elm));
}

/// Register `listen{Type}` and the incoming port. The payload is one JSON text frame.
#[instrument(skip(module), fields(elm = T::elm_type()))]
pub fn contribute_listen<T: Elm + ElmDecode>(module: &mut Module) {
    T::contribute_decoder(module);
    let elm = T::elm_type();
    let name = format!("listen{elm}");
    if !module.mark(&name) {
        return;
    }
    module.push_definition(&listen_source::<T>(&elm));
}

fn camel_port(elm: &str) -> String {
    let mut chars = elm.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn send_source<T: ElmEncode>(elm: &str) -> String {
    let port = format!("{}Outgoing", camel_port(elm));
    let encoder = T::encoder_type();
    format!(
        "\
port {port} : String -> Cmd msg

send{elm} : {elm} -> Cmd msg
send{elm} message =
    {port} (Json.Encode.encode 0 ({encoder} message))
"
    )
}

fn listen_source<T: ElmDecode>(elm: &str) -> String {
    let port = format!("{}Incoming", camel_port(elm));
    let decoder = T::decoder_type();
    format!(
        "\
port {port} : (String -> msg) -> Sub msg

listen{elm} : (Result Json.Decode.Error {elm} -> msg) -> Sub msg
listen{elm} tagger =
    {port} (\\raw -> tagger (Json.Decode.decodeString {decoder} raw))
"
    )
}
