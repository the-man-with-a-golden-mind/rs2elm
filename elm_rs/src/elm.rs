//! Contains the `Elm` trait.

#[cfg(feature = "derive")]
pub use rs2elm_derive::Elm;

/// Used to represent Rust types in Elm.
pub trait Elm {
    /// The name of the type in Elm.
    fn elm_type() -> String;
    /// The definition of the type in Elm. None for types already defined in Elm.
    fn elm_definition() -> Option<String>;
    /// Elm constructors this type adds to the module. Empty for type aliases and builtins.
    fn elm_constructors() -> &'static [&'static str] {
        &[]
    }

    /// Register this type and the types it mentions.
    ///
    /// The default records a builtin. Containers override it and forward to their parameters.
    /// Derived types mark the Elm constructor first, then their fields, then their own definition.
    fn contribute(module: &mut crate::export::Module) {
        let name = Self::elm_type();
        if !module.mark(&name) {
            return;
        }
        for constructor in Self::elm_constructors() {
            module.note_constructor(constructor, &name);
        }
        if let Some(definition) = Self::elm_definition() {
            module.push_definition(&definition);
        }
    }
}

impl<T> Elm for (T,)
where
    T: Elm,
{
    fn elm_type() -> String {
        T::elm_type()
    }

    fn elm_definition() -> Option<String> {
        None
    }

    fn contribute(module: &mut crate::export::Module) {
        T::contribute(module);
    }
}

impl<T, U> Elm for (T, U)
where
    T: Elm,
    U: Elm,
{
    fn elm_type() -> String {
        ::std::format!("( {}, {} )", T::elm_type(), U::elm_type())
    }

    fn elm_definition() -> Option<String> {
        None
    }

    fn contribute(module: &mut crate::export::Module) {
        T::contribute(module);
        U::contribute(module);
    }
}

impl<T, U, V> Elm for (T, U, V)
where
    T: Elm,
    U: Elm,
    V: Elm,
{
    fn elm_type() -> String {
        ::std::format!(
            "( {}, {}, {} )",
            T::elm_type(),
            U::elm_type(),
            V::elm_type()
        )
    }

    fn elm_definition() -> Option<String> {
        None
    }

    fn contribute(module: &mut crate::export::Module) {
        T::contribute(module);
        U::contribute(module);
        V::contribute(module);
    }
}

impl<T: Elm + ToOwned + ?Sized> Elm for std::borrow::Cow<'_, T> {
    fn elm_type() -> String {
        T::elm_type()
    }

    fn elm_definition() -> Option<String> {
        T::elm_definition()
    }

    fn contribute(module: &mut crate::export::Module) {
        T::contribute(module);
    }
}

impl<T, const U: usize> Elm for [T; U]
where
    T: Elm,
{
    fn elm_type() -> String {
        <[T]>::elm_type()
    }

    fn elm_definition() -> Option<String> {
        <[T]>::elm_definition()
    }

    fn contribute(module: &mut crate::export::Module) {
        T::contribute(module);
    }
}

impl Elm for std::time::Duration {
    fn elm_type() -> String {
        "Duration".to_string()
    }

    fn elm_definition() -> Option<String> {
        Some(
            "\
type alias Duration =
    { secs : Int
    , nanos : Int
    }
"
            .to_string(),
        )
    }
}

impl<T: Elm, E: Elm> Elm for Result<T, E> {
    fn elm_type() -> String {
        ::std::format!("Result ({}) ({})", E::elm_type(), T::elm_type())
    }

    fn elm_definition() -> Option<String> {
        None
    }

    fn contribute(module: &mut crate::export::Module) {
        E::contribute(module);
        T::contribute(module);
    }
}

impl Elm for std::time::SystemTime {
    fn elm_type() -> String {
        "SystemTime".to_string()
    }

    fn elm_definition() -> Option<String> {
        Some(
            "\
type alias SystemTime =
    { secs_since_epoch : Int
    , nanos_since_epoch : Int
    }
"
            .to_string(),
        )
    }
}

macro_rules! impl_builtin {
    ($rust_type: ty, $elm_type: expr) => {
        impl Elm for $rust_type {
            fn elm_type() -> String {
                $elm_type.to_string()
            }

            fn elm_definition() -> Option<String> {
                None
            }
        }
    };
}

macro_rules! impl_builtin_container {
    ($rust_type: ty, $elm_name: expr) => {
        impl<T: Elm> Elm for $rust_type {
            fn elm_type() -> String {
                ::std::format!("{} ({})", $elm_name, T::elm_type())
            }

            fn elm_definition() -> Option<String> {
                None
            }

            fn contribute(module: &mut crate::export::Module) {
                T::contribute(module);
            }
        }
    };
}

macro_rules! impl_builtin_map {
    ($rust_type: ty) => {
        impl<T: Elm> Elm for $rust_type {
            fn elm_type() -> String {
                ::std::format!("Dict String ({})", T::elm_type())
            }

            fn elm_definition() -> Option<String> {
                None
            }

            fn contribute(module: &mut crate::export::Module) {
                T::contribute(module);
            }
        }
    };
}

macro_rules! impl_builtin_ptr {
    ($rust_type: ty) => {
        impl<T: Elm + ?Sized> Elm for $rust_type {
            fn elm_type() -> String {
                ::std::format!("{}", T::elm_type())
            }

            fn elm_definition() -> Option<String> {
                T::elm_definition()
            }

            fn contribute(module: &mut crate::export::Module) {
                T::contribute(module);
            }
        }
    };
}

impl_builtin!((), "()");
impl_builtin_ptr!(&'_ T);
impl_builtin_ptr!(&'_ mut T);
impl_builtin_ptr!(std::sync::Arc<T>);
impl_builtin!(std::sync::atomic::AtomicBool, "Bool");
impl_builtin!(std::sync::atomic::AtomicU8, "Int");
impl_builtin!(std::sync::atomic::AtomicU16, "Int");
impl_builtin!(std::sync::atomic::AtomicU32, "Int");
impl_builtin!(std::sync::atomic::AtomicU64, "Int");
impl_builtin!(std::sync::atomic::AtomicUsize, "Int");
impl_builtin!(std::sync::atomic::AtomicI8, "Int");
impl_builtin!(std::sync::atomic::AtomicI16, "Int");
impl_builtin!(std::sync::atomic::AtomicI32, "Int");
impl_builtin!(std::sync::atomic::AtomicI64, "Int");
impl_builtin!(std::sync::atomic::AtomicIsize, "Int");
impl_builtin_map!(std::collections::BTreeMap<String,T>);
impl_builtin_container!(std::collections::BTreeSet<T>, "List");
impl_builtin_ptr!(Box<T>);
impl_builtin_ptr!(std::cell::Cell<T>);
impl_builtin_map!(std::collections::HashMap<String,T>);
impl_builtin_container!(std::collections::HashSet<T>, "List");
impl_builtin!(std::net::IpAddr, "String");
impl_builtin!(std::net::Ipv4Addr, "String");
impl_builtin!(std::net::Ipv6Addr, "String");
impl_builtin!(std::net::SocketAddr, "String");
impl_builtin!(std::net::SocketAddrV4, "String");
impl_builtin!(std::net::SocketAddrV6, "String");
impl_builtin_container!(std::collections::LinkedList<T>, "List");
impl_builtin_ptr!(std::sync::Mutex<T>);
impl_builtin!(std::num::NonZeroU8, "Int");
impl_builtin!(std::num::NonZeroU16, "Int");
impl_builtin!(std::num::NonZeroU32, "Int");
impl_builtin!(std::num::NonZeroU64, "Int");
impl_builtin!(std::num::NonZeroU128, "Int");
impl_builtin!(std::num::NonZeroUsize, "Int");
impl_builtin!(std::num::NonZeroI8, "Int");
impl_builtin!(std::num::NonZeroI16, "Int");
impl_builtin!(std::num::NonZeroI32, "Int");
impl_builtin!(std::num::NonZeroI64, "Int");
impl_builtin!(std::num::NonZeroI128, "Int");
impl_builtin!(std::num::NonZeroIsize, "Int");

impl_builtin_container!(Option<T>, "Maybe");
impl_builtin!(std::path::Path, "String");
impl_builtin!(std::path::PathBuf, "String");
impl<T> Elm for std::marker::PhantomData<T> {
    fn elm_type() -> String {
        "()".to_string()
    }

    fn elm_definition() -> Option<String> {
        None
    }
}
impl_builtin_ptr!(std::rc::Rc<T>);
impl_builtin_ptr!(std::cell::RefCell<T>);
impl_builtin_ptr!(std::sync::RwLock<T>);
impl_builtin!(String, "String");
impl_builtin_container!(Vec<T>, "List");
impl_builtin_container!([T], "List");
impl_builtin!(bool, "Bool");
impl_builtin!(char, "String");
impl_builtin!(u8, "Int");
impl_builtin!(u16, "Int");
impl_builtin!(u32, "Int");
impl_builtin!(u64, "Int");
impl_builtin!(u128, "Int");
impl_builtin!(usize, "Int");
impl_builtin!(i8, "Int");
impl_builtin!(i16, "Int");
impl_builtin!(i32, "Int");
impl_builtin!(i64, "Int");
impl_builtin!(i128, "Int");
impl_builtin!(isize, "Int");
impl_builtin!(f32, "Float");
impl_builtin!(f64, "Float");
impl_builtin!(str, "String");

#[cfg(feature = "uuid")]
impl_builtin!(uuid::Uuid, "String");

#[cfg(feature = "chrono")]
impl_builtin!(chrono::NaiveTime, "String");
#[cfg(feature = "chrono")]
impl_builtin!(chrono::NaiveDate, "String");
#[cfg(feature = "chrono")]
impl_builtin!(chrono::NaiveDateTime, "String");
#[cfg(feature = "chrono")]
impl<T: chrono::TimeZone> Elm for chrono::DateTime<T> {
    fn elm_type() -> String {
        String::elm_type()
    }

    fn elm_definition() -> Option<String> {
        String::elm_definition()
    }
}
