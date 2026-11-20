//! Contains the `ElmQuery` trait.

#[cfg(feature = "derive")]
pub use rs2elm_derive::ElmQuery;
#[cfg(feature = "derive")]
pub use rs2elm_derive::ElmQueryField;

/// Used to generate URL encoded key-value pairs in Elm.
///
/// The generated list is the body of an `application/x-www-form-urlencoded` form.
/// `Option` contributes nothing when the value is `Nothing`. Lists repeat one key.
/// A nested `ElmQuery` writes bracket keys such as `thumbnail[width]`.
/// `bool` and floats are `Url.Builder.string` (`"true"` / `"false"`, `String.fromFloat`).
///
/// `elm_form` reads that body back from `List (String, String)`. A field marked
/// `#[rs2elm(index)]` numbers list elements (`shots[0][width]`) instead of repeating the key.
pub trait ElmQuery {
    /// Generates an Elm function that creates a `List Url.Builder.QueryParameter`.
    fn elm_query() -> String;

    /// Generates `urlDecode{Type}`, which reads form pairs into this value.
    fn elm_form() -> String {
        ::std::string::String::new()
    }

    /// Elm expression that encodes `value` under an Elm string expression `prefix`.
    ///
    /// `prefix` is `""` at the top level and `thumbnail` when this value is nested,
    /// so a field `width` becomes `thumbnail[width]`. The derive fills this in.
    fn query_nested(prefix: &str, value: &str) -> String {
        let _ = (prefix, value);
        ::std::string::String::new()
    }

    /// Elm expression of type `String -> List (String, String) -> Result String Self`.
    fn form_parser() -> String {
        ::std::string::String::new()
    }

    /// Register this query function and the fields it calls.
    fn contribute_query(module: &mut crate::export::Module) {
        let _ = module;
    }
}

impl<T: ElmQuery + ?Sized> ElmQuery for &'_ T {
    fn elm_query() -> String {
        T::elm_query()
    }

    fn query_nested(prefix: &str, value: &str) -> String {
        T::query_nested(prefix, value)
    }

    fn elm_form() -> String {
        T::elm_form()
    }

    fn form_parser() -> String {
        T::form_parser()
    }

    fn contribute_query(module: &mut crate::export::Module) {
        T::contribute_query(module);
    }
}

impl<T: ElmQuery + ?Sized> ElmQuery for &'_ mut T {
    fn elm_query() -> String {
        T::elm_query()
    }

    fn query_nested(prefix: &str, value: &str) -> String {
        T::query_nested(prefix, value)
    }

    fn elm_form() -> String {
        T::elm_form()
    }

    fn form_parser() -> String {
        T::form_parser()
    }

    fn contribute_query(module: &mut crate::export::Module) {
        T::contribute_query(module);
    }
}

/// Used to generate the fields for `ElmQuery::elm_query`.
pub trait ElmQueryField {
    /// The `Url.Builder` type of the field (either `Url.Builder.string` or `Url.Builder.int`).
    fn query_field_type() -> &'static str;
    /// The name of the Elm function used to transform it a String or Int for the `query_field_type` (usually `identity`).
    fn query_field_encoder_name() -> &'static str {
        "identity"
    }
    /// If the type needs a custom encoder, this function generates its definition.
    fn query_field_encoder_definition() -> Option<String> {
        None
    }

    /// Elm function of type `String -> Result String Self` used by form decoding.
    fn query_field_form_value() -> &'static str {
        "elmRsFormValueString"
    }

    /// Custom form decoder for a derived unit enum. Builtins use `query_field_form_value`.
    fn query_field_decoder_definition() -> Option<String> {
        None
    }

    /// Register this query-field encoder. Builtins have nothing to register.
    fn contribute_query_field(module: &mut crate::export::Module) {
        let _ = module;
    }
}

impl<T: ElmQueryField + ?Sized> ElmQueryField for &'_ T {
    fn query_field_type() -> &'static str {
        T::query_field_type()
    }

    fn query_field_encoder_name() -> &'static str {
        T::query_field_encoder_name()
    }

    fn query_field_encoder_definition() -> Option<String> {
        T::query_field_encoder_definition()
    }

    fn query_field_form_value() -> &'static str {
        T::query_field_form_value()
    }

    fn query_field_decoder_definition() -> Option<String> {
        T::query_field_decoder_definition()
    }

    fn contribute_query_field(module: &mut crate::export::Module) {
        T::contribute_query_field(module);
    }
}

impl<T: ElmQueryField + ?Sized> ElmQueryField for &'_ mut T {
    fn query_field_type() -> &'static str {
        T::query_field_type()
    }

    fn query_field_encoder_name() -> &'static str {
        T::query_field_encoder_name()
    }

    fn query_field_encoder_definition() -> Option<String> {
        T::query_field_encoder_definition()
    }

    fn query_field_form_value() -> &'static str {
        T::query_field_form_value()
    }

    fn query_field_decoder_definition() -> Option<String> {
        T::query_field_decoder_definition()
    }

    fn contribute_query_field(module: &mut crate::export::Module) {
        T::contribute_query_field(module);
    }
}

macro_rules! impl_for {
    ($e:expr, $form:expr, $($t:ty),+) => {
        $(
            impl ElmQueryField for $t {
                fn query_field_type() -> &'static str {
                    $e
                }

                fn query_field_form_value() -> &'static str {
                    $form
                }
            }
        )*
    };
}

impl_for!(
    "Url.Builder.string",
    "elmRsFormValueString",
    String,
    str,
    char,
    std::path::Path,
    std::path::PathBuf,
    std::net::IpAddr,
    std::net::Ipv4Addr,
    std::net::Ipv6Addr,
    std::net::SocketAddr,
    std::net::SocketAddrV4,
    std::net::SocketAddrV6
);
#[cfg(feature = "uuid")]
impl_for!("Url.Builder.string", "elmRsFormValueString", uuid::Uuid);
#[cfg(feature = "chrono")]
impl_for!(
    "Url.Builder.string",
    "elmRsFormValueString",
    chrono::NaiveTime,
    chrono::NaiveDate,
    chrono::NaiveDateTime
);
#[cfg(feature = "chrono")]
impl<T: chrono::TimeZone> ElmQueryField for chrono::DateTime<T> {
    fn query_field_type() -> &'static str {
        "Url.Builder.string"
    }
}

impl ElmQueryField for bool {
    fn query_field_type() -> &'static str {
        "Url.Builder.string"
    }

    fn query_field_encoder_name() -> &'static str {
        r#"(\value -> if value then "true" else "false")"#
    }

    fn query_field_form_value() -> &'static str {
        "elmRsFormValueBool"
    }
}

impl ElmQueryField for f32 {
    fn query_field_type() -> &'static str {
        "Url.Builder.string"
    }

    fn query_field_encoder_name() -> &'static str {
        "String.fromFloat"
    }

    fn query_field_form_value() -> &'static str {
        "elmRsFormValueFloat"
    }
}

impl ElmQueryField for f64 {
    fn query_field_type() -> &'static str {
        "Url.Builder.string"
    }

    fn query_field_encoder_name() -> &'static str {
        "String.fromFloat"
    }

    fn query_field_form_value() -> &'static str {
        "elmRsFormValueFloat"
    }
}

impl_for!(
    "Url.Builder.int",
    "elmRsFormValueInt",
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    std::sync::atomic::AtomicU8,
    std::sync::atomic::AtomicU16,
    std::sync::atomic::AtomicU32,
    std::sync::atomic::AtomicU64,
    std::sync::atomic::AtomicUsize,
    std::sync::atomic::AtomicI8,
    std::sync::atomic::AtomicI16,
    std::sync::atomic::AtomicI32,
    std::sync::atomic::AtomicI64,
    std::sync::atomic::AtomicIsize,
    std::num::NonZeroU8,
    std::num::NonZeroU16,
    std::num::NonZeroU32,
    std::num::NonZeroU64,
    std::num::NonZeroU128,
    std::num::NonZeroUsize,
    std::num::NonZeroI8,
    std::num::NonZeroI16,
    std::num::NonZeroI32,
    std::num::NonZeroI64,
    std::num::NonZeroI128,
    std::num::NonZeroIsize
);

/// Picks a nested `ElmQuery` over an `ElmQueryField` at the derive's call site.
///
/// A generic helper cannot do this: method resolution only sees the bounds on
/// that helper. The derive emits the call next to the concrete field type.
#[doc(hidden)]
pub struct QueryDispatch<T>(pub std::marker::PhantomData<T>);

#[doc(hidden)]
pub trait QueryLeaf {
    /// Elm expression that encodes `value` under the key expression `key`.
    fn query_list(&self, key: &str, value: &str) -> String;
    /// Elm parser of type `String -> List (String, String) -> Result String Self`.
    fn form_parser(&self) -> String;
    /// Register the query codec this leaf calls.
    fn query_contribute(&self, module: &mut crate::export::Module);
}

impl<T> QueryLeaf for &&&QueryDispatch<T>
where
    T: crate::Elm + ElmQuery,
{
    fn query_list(&self, key: &str, value: &str) -> String {
        T::query_nested(key, value)
    }

    fn form_parser(&self) -> String {
        T::form_parser()
    }

    fn query_contribute(&self, module: &mut crate::export::Module) {
        T::contribute_query(module);
    }
}

impl<T: ElmQueryField> QueryLeaf for &&QueryDispatch<T> {
    fn query_list(&self, key: &str, value: &str) -> String {
        ::std::format!(
            "[ {} {} ({} {}) ]",
            T::query_field_type(),
            key,
            T::query_field_encoder_name(),
            value
        )
    }

    fn form_parser(&self) -> String {
        ::std::format!("(elmRsFormText {})", T::query_field_form_value())
    }

    fn query_contribute(&self, module: &mut crate::export::Module) {
        T::contribute_query_field(module);
    }
}

/// Elm helpers shared by every generated form decoder.
///
/// Top-level declarations are separated by a blank line. elm repl 0.19.1 crashes
/// (`Map.!`) when they are not.
#[doc(hidden)]
pub fn form_helpers() -> &'static str {
    r#"elmRsFormFind : String -> List (String, String) -> Maybe String
elmRsFormFind key pairs =
    List.head
        (List.filterMap
            (\(name, text) ->
                if name == key then
                    Just text
                else
                    Nothing
            )
            pairs
        )


elmRsFormValueInt : String -> Result String Int
elmRsFormValueInt text =
    case String.toInt text of
        Just value ->
            Ok value
        Nothing ->
            Err "invalid int"


elmRsFormValueFloat : String -> Result String Float
elmRsFormValueFloat text =
    case String.toFloat text of
        Just value ->
            Ok value
        Nothing ->
            Err "invalid float"


elmRsFormValueBool : String -> Result String Bool
elmRsFormValueBool text =
    if text == "true" then
        Ok True
    else if text == "false" then
        Ok False
    else
        Err "invalid bool"


elmRsFormValueString : String -> Result String String
elmRsFormValueString text =
    Ok text


elmRsFormText : (String -> Result String a) -> String -> List (String, String) -> Result String a
elmRsFormText decode key pairs =
    case elmRsFormFind key pairs of
        Nothing ->
            Err ("missing " ++ key)
        Just text ->
            decode text


elmRsFormOptional : (String -> List (String, String) -> Result String a) -> String -> List (String, String) -> Result String (Maybe a)
elmRsFormOptional decode prefix pairs =
    if List.any (\(key, _) -> key == prefix || String.startsWith (prefix ++ "[") key) pairs then
        Result.map Just (decode prefix pairs)
    else
        Ok Nothing


elmRsFormSequence : List (Result String a) -> Result String (List a)
elmRsFormSequence results =
    List.foldr
        (\item acc ->
            case acc of
                Err _ ->
                    acc
                Ok rest ->
                    case item of
                        Err err ->
                            Err err
                        Ok value ->
                            Ok (value :: rest)
        )
        (Ok [])
        results


elmRsFormChunkStep : String -> ( String, String ) -> List (List ( String, String )) -> List (List ( String, String ))
elmRsFormChunkStep prefix (key, value) chunks =
    if not (key == prefix || String.startsWith (prefix ++ "[") key) then
        chunks
    else
        case chunks of
            [] ->
                [ [ ( key, value ) ] ]
            current :: older ->
                if List.any (\(seen, _) -> seen == key) current then
                    [ ( key, value ) ] :: current :: older
                else
                    (( key, value ) :: current) :: older


elmRsFormChunks : String -> List (String, String) -> List (List (String, String))
elmRsFormChunks prefix pairs =
    List.reverse (List.map List.reverse (List.foldl (elmRsFormChunkStep prefix) [] pairs))


elmRsFormRepeated : (String -> List (String, String) -> Result String a) -> String -> List (String, String) -> Result String (List a)
elmRsFormRepeated decode prefix pairs =
    elmRsFormSequence (List.map (\chunk -> decode prefix chunk) (elmRsFormChunks prefix pairs))


elmRsFormIndex : String -> String -> Maybe Int
elmRsFormIndex prefix key =
    let
        opener = prefix ++ "["
    in
    if String.startsWith opener key then
        let
            rest = String.dropLeft (String.length opener) key
        in
        case String.indexes "]" rest of
            offset :: _ ->
                String.toInt (String.left offset rest)
            [] ->
                Nothing
    else
        Nothing


elmRsFormIndexes : String -> List (String, String) -> List Int
elmRsFormIndexes prefix pairs =
    List.foldl
        (\(key, _) found ->
            case elmRsFormIndex prefix key of
                Just index ->
                    if List.member index found then
                        found
                    else
                        found ++ [ index ]
                Nothing ->
                    found
        )
        []
        pairs
        |> List.sort


elmRsFormIndexed : (String -> List (String, String) -> Result String a) -> String -> List (String, String) -> Result String (List a)
elmRsFormIndexed decode prefix pairs =
    let
        indexes = elmRsFormIndexes prefix pairs
    in
    case List.maximum indexes of
        Nothing ->
            Ok []
        Just end ->
            elmRsFormSequence
                (List.map
                    (\index ->
                        if List.member index indexes then
                            decode (prefix ++ "[" ++ String.fromInt index ++ "]") pairs
                        else
                            Err ("missing " ++ prefix ++ "[" ++ String.fromInt index ++ "]")
                    )
                    (List.range 0 end))


elmRsFormIndexedMaybe : (String -> List (String, String) -> Result String a) -> String -> List (String, String) -> Result String (List (Maybe a))
elmRsFormIndexedMaybe decode prefix pairs =
    let
        indexes = elmRsFormIndexes prefix pairs
    in
    case List.maximum indexes of
        Nothing ->
            Ok []
        Just end ->
            elmRsFormSequence
                (List.map
                    (\index ->
                        if List.member index indexes then
                            Result.map Just (decode (prefix ++ "[" ++ String.fromInt index ++ "]") pairs)
                        else
                            Ok Nothing
                    )
                    (List.range 0 end))
"#
}
