use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, process::Command};

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ClientMsg {
    Ping,
    Say { text: String },
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
enum ServerMsg {
    Pong,
    Said { text: String },
}

#[test]
fn sockets_are_ports_around_json() {
    let mut target = Vec::new();
    crate::export!("Bindings", &mut target, {
        socket_send: [ClientMsg],
        socket_listen: [ServerMsg],
    })
    .unwrap();
    let source = String::from_utf8(target).unwrap();
    assert!(source.contains("port module Bindings"), "{source}");
    assert!(
        source.contains("port clientMsgOutgoing : String -> Cmd msg"),
        "{source}"
    );
    assert!(
        source.contains("port serverMsgIncoming : (String -> msg) -> Sub msg"),
        "{source}"
    );
    assert!(
        source.contains("sendClientMsg : ClientMsg -> Cmd msg"),
        "{source}"
    );
    assert!(
        source.contains("listenServerMsg : (Result Json.Decode.Error ServerMsg -> msg) -> Sub msg"),
        "{source}"
    );
    assert!(source.contains("clientMsgEncoder"), "{source}");
    assert!(source.contains("serverMsgDecoder"), "{source}");

    let dir = std::env::temp_dir().join("rs2elm_socket");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../elm-test");
    fs::copy(root.join("elm.json"), dir.join("elm.json")).unwrap();
    fs::write(dir.join("src/Bindings.elm"), &source).unwrap();
    fs::write(
        dir.join("src/Main.elm"),
        r#"
module Main exposing (main)

import Bindings exposing (ClientMsg(..), ServerMsg(..))
import Platform

main : Program () () ()
main =
    Platform.worker
        { init = \_ -> ( (), Bindings.sendClientMsg (Say { text = "hi" }) )
        , update = \_ model -> ( model, Cmd.none )
        , subscriptions = \_ -> Bindings.listenServerMsg (\_ -> ())
        }
"#,
    )
    .unwrap();
    let output = Command::new("elm")
        .args(["make", "src/Main.elm", "--output=/dev/null"])
        .current_dir(&dir)
        .output()
        .expect("elm is installed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "elm make failed\nstdout:\n{stdout}\nstderr:\n{stderr}\nsource:\n{source}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_second_socket_registration_keeps_one_port() {
    let mut module = crate::export::Module::new();
    crate::socket::contribute_send::<ClientMsg>(&mut module);
    crate::socket::contribute_send::<ClientMsg>(&mut module);
    crate::socket::contribute_listen::<ServerMsg>(&mut module);
    crate::socket::contribute_listen::<ServerMsg>(&mut module);
    let source = module.finish("Sockets").unwrap();
    assert_eq!(
        source.matches("port clientMsgOutgoing").count(),
        1,
        "{source}"
    );
    assert_eq!(
        source.matches("port serverMsgIncoming").count(),
        1,
        "{source}"
    );
}

struct Blank;

impl Elm for Blank {
    fn elm_type() -> String {
        String::new()
    }

    fn elm_definition() -> Option<String> {
        None
    }
}

impl ElmEncode for Blank {
    fn encoder_type() -> String {
        "Json.Encode.int".to_string()
    }

    fn encoder_definition() -> Option<String> {
        None
    }
}

#[test]
fn an_empty_elm_name_has_an_unprefixed_port() {
    let mut module = crate::export::Module::new();
    crate::socket::contribute_send::<Blank>(&mut module);
    let source = module.finish("Blank").unwrap();
    assert!(
        source.contains("port Outgoing : String -> Cmd msg"),
        "{source}"
    );
    assert!(source.contains("send :  -> Cmd msg"), "{source}");
}
