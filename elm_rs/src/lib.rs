#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    rust_2018_idioms,
    unused_must_use,
    unreachable_pub,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    single_use_lifetimes,
    rustdoc::broken_intra_doc_links
)]

mod elm;
mod elm_decode;
mod elm_encode;
mod elm_query;
#[doc(hidden)]
pub mod export;
#[doc(hidden)]
pub mod socket;
#[cfg(test)]
mod test;
#[cfg(feature = "time")]
mod time_elm;

#[cfg(test)]
extern crate self as rs2elm;

pub use self::{
    elm::Elm,
    elm_decode::ElmDecode,
    elm_encode::ElmEncode,
    elm_query::{ElmQuery, ElmQueryField},
};
#[doc(hidden)]
pub use elm_query::{QueryDispatch, QueryLeaf, form_helpers};
/// Tracing macros for generated-code paths. `#[instrument]` expands to `::tracing::`.
pub use tracing::{debug, error, info, instrument, trace, warn};

#[macro_export]
/// Writes an Elm module to the target.
///
/// The module imports `elm/json` when it encodes or decodes JSON, and `elm/url`
/// when it builds a query. Install those packages before compiling the file.
///
/// `socket_send` and `socket_listen` turn the file into a `port module`. Each
/// value is one JSON text frame. JavaScript owns the WebSocket.
///
/// # Example
/// ```no_run
#[doc = include_str!("../examples/example.rs")]
/// ```
macro_rules! export {
    ($name: expr, $target: expr, {
        $(
            encoders:   [ $($encode: ty),*   $(,)? ] $(,)?
        )?
        $(
            decoders:   [ $($decode: ty),*   $(,)? ] $(,)?
        )?
        $(
            queries:    [ $($query: ty),*    $(,)? ] $(,)?
        )?
        $(
            query_fields:    [ $($query_field: ty),*    $(,)? ] $(,)?
        )?
        $(
            socket_send: [ $($send: ty),* $(,)? ] $(,)?
        )?
        $(
            socket_listen: [ $($listen: ty),* $(,)? ] $(,)?
        )?
    }) => {
        {
            fn _export(name: &::std::primitive::str, target: &mut impl ::std::io::Write) -> ::std::result::Result<(), ::std::io::Error> {
                let mut module = $crate::export::Module::new();
                $($(
                    <$encode as $crate::ElmEncode>::contribute_encoder(&mut module);
                )*)?
                $($(
                    <$decode as $crate::ElmDecode>::contribute_decoder(&mut module);
                )*)?
                $($(
                    <$query as $crate::ElmQuery>::contribute_query(&mut module);
                )*)?
                $($(
                    <$query_field as $crate::ElmQueryField>::contribute_query_field(&mut module);
                )*)?
                $($(
                    $crate::socket::contribute_send::<$send>(&mut module);
                )*)?
                $($(
                    $crate::socket::contribute_listen::<$listen>(&mut module);
                )*)?
                let rendered = module.finish(name)?;
                target.write_all(rendered.as_bytes())?;
                ::std::result::Result::Ok(())
            }
            _export($name, $target)
        }
    };
}
