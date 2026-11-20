use crate::{Elm, ElmDecode, ElmEncode};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    collections::*,
    hash::Hash,
    num::*,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex, RwLock, atomic::*},
    time::{Duration, SystemTime},
};

#[derive(Debug, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct Types<T: Copy + Hash + Eq> {
    t: T,
    unit: (),
    one: (u8,),
    two: (u8, u8),
    three: (u8, u8, u8),
    arc: Arc<T>,
    abool: AtomicBool,
    ai8: AtomicI8,
    ai16: AtomicI16,
    ai32: AtomicI32,
    ai64: AtomicI64,
    aisize: AtomicIsize,
    au8: AtomicU8,
    au16: AtomicU16,
    au32: AtomicU32,
    au64: AtomicU64,
    ausize: AtomicUsize,
    btreemap: BTreeMap<String, T>,
    btreeset: BTreeSet<u8>,
    b: Box<T>,
    cell: Cell<T>,
    cow: Cow<'static, u8>,
    hashmap: HashMap<String, T>,
    hashset: HashSet<T>,
    linkedlist: LinkedList<T>,
    mutex: Mutex<T>,
    nu8: NonZeroU8,
    nu16: NonZeroU16,
    nu32: NonZeroU32,
    nu64: NonZeroU64,
    nusize: NonZeroUsize,
    ni8: NonZeroI8,
    ni16: NonZeroI16,
    ni32: NonZeroI32,
    ni64: NonZeroI64,
    nisize: NonZeroIsize,
    option_some: Option<T>,
    option_none: Option<T>,
    pathbuf: PathBuf,
    rc: Rc<T>,
    refcell: RefCell<T>,
    result: Result<T, T>,
    rwlock: RwLock<T>,
    string: String,
    vec: Vec<T>,
    array: [T; 2],
    bool: bool,
    f32: f32,
    f64: f64,
    u8: u8,
    u16: u16,
    u32: u32,
    u64: u64,
    usize: usize,
    i8: i8,
    i16: i16,
    i32: i32,
    i64: i64,
    isize: isize,
}

#[test]
fn types() {
    super::test_json_without_eq(
        &Types {
            t: 0u8,
            unit: (),
            one: (0,),
            two: (0, 0),
            three: (0, 0, 0),
            arc: Arc::new(0),
            abool: AtomicBool::default(),
            ai8: AtomicI8::default(),
            ai16: AtomicI16::default(),
            ai32: AtomicI32::default(),
            ai64: AtomicI64::default(),
            aisize: AtomicIsize::default(),
            au8: AtomicU8::default(),
            au16: AtomicU16::default(),
            au32: AtomicU32::default(),
            au64: AtomicU64::default(),
            ausize: AtomicUsize::default(),
            btreemap: BTreeMap::default(),
            btreeset: BTreeSet::default(),
            b: Box::new(0),
            cell: Cell::new(0),
            cow: Cow::Owned(0),
            hashmap: HashMap::default(),
            hashset: HashSet::default(),
            linkedlist: LinkedList::default(),
            mutex: Mutex::new(0),
            nu8: NonZeroU8::new(1).unwrap(),
            nu16: NonZeroU16::new(1).unwrap(),
            nu32: NonZeroU32::new(1).unwrap(),
            nu64: NonZeroU64::new(1).unwrap(),
            nusize: NonZeroUsize::new(1).unwrap(),
            ni8: NonZeroI8::new(1).unwrap(),
            ni16: NonZeroI16::new(1).unwrap(),
            ni32: NonZeroI32::new(1).unwrap(),
            ni64: NonZeroI64::new(1).unwrap(),
            nisize: NonZeroIsize::new(1).unwrap(),
            option_some: Some(0),
            option_none: None,
            pathbuf: PathBuf::default(),
            rc: Rc::new(0),
            refcell: RefCell::new(0),
            result: Err(0),
            rwlock: RwLock::new(0),
            string: "0".to_string(),
            vec: vec![0, 0],
            array: [0, 0],
            bool: false,
            f32: 0.0,
            f64: 0.0,
            u8: 0,
            u16: 0,
            u32: 0,
            u64: 0,
            usize: 0,
            i8: 0,
            i16: 0,
            i32: 0,
            i64: 0,
            isize: 0,
        },
        &::std::format!(
            "\
{}

{}

",
            Result::<(), ()>::encoder_definition().unwrap(),
            Result::<(), ()>::decoder_definition().unwrap(),
        ),
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct StdScalars {
    ch: char,
    phantom: std::marker::PhantomData<u8>,
    ip: std::net::IpAddr,
    ipv4: std::net::Ipv4Addr,
    ipv6: std::net::Ipv6Addr,
    sock: std::net::SocketAddr,
    sock4: std::net::SocketAddrV4,
    sock6: std::net::SocketAddrV6,
    big_i: i64,
    big_u: u64,
    pair: [u8; 2],
}

#[test]
fn std_scalars_match_serde_json() {
    super::test_json(StdScalars {
        ch: 'a',
        phantom: std::marker::PhantomData,
        ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        ipv4: std::net::Ipv4Addr::LOCALHOST,
        ipv6: std::net::Ipv6Addr::LOCALHOST,
        sock: std::net::SocketAddr::from(([127, 0, 0, 1], 80)),
        sock4: std::net::SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 80),
        sock6: std::net::SocketAddrV6::new(std::net::Ipv6Addr::LOCALHOST, 80, 0, 0),
        big_i: 9_007_199_254_740_991,
        big_u: 9_007_199_254_740_991,
        pair: [1, 2],
    });
}

#[test]
fn char_decoder_rejects_multiple_characters() {
    let decoder = StdScalars::decoder_type();
    let json = r#"{"ch":"ab","phantom":null,"ip":"127.0.0.1","ipv4":"127.0.0.1","ipv6":"::1","sock":"127.0.0.1:80","sock4":"127.0.0.1:80","sock6":"[::1]:80","big_i":0,"big_u":0,"pair":[1,2]}"#.replace('"', "\\\"");
    let rejected = super::elm_text::<StdScalars>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(
        rejected.contains("expected a single character"),
        "{rejected}"
    );
}

#[test]
fn array_decoder_rejects_the_wrong_length() {
    let decoder = StdScalars::decoder_type();
    let json = r#"{"ch":"a","phantom":null,"ip":"127.0.0.1","ipv4":"127.0.0.1","ipv6":"::1","sock":"127.0.0.1:80","sock4":"127.0.0.1:80","sock6":"[::1]:80","big_i":0,"big_u":0,"pair":[1]}"#.replace('"', "\\\"");
    let rejected = super::elm_text::<StdScalars>(&format!(
        r#"    case Json.Decode.decodeString {decoder} "{json}" of
        Ok _ ->
            "accepted"
        Err err ->
            Json.Decode.errorToString err"#
    ));
    assert!(
        rejected.contains("expected a list of length 2"),
        "{rejected}"
    );
}

/// `time` values as `serde_json` writes them without `serde-human-readable`.
#[cfg(feature = "time")]
#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TimeSample {
    date: time::Date,
    clock: time::Time,
    primitive: time::PrimitiveDateTime,
    offset: time::OffsetDateTime,
    span: time::Duration,
    earlier: time::OffsetDateTime,
    behind: time::Duration,
}

#[cfg(feature = "time")]
fn sample_time() -> TimeSample {
    use time::{Date, Duration, Month, PrimitiveDateTime, Time, UtcOffset};
    let date = Date::from_calendar_date(2020, Month::January, 2).unwrap();
    let clock = Time::from_hms_nano(3, 4, 5, 6).unwrap();
    let primitive = PrimitiveDateTime::new(date, clock);
    let offset = primitive.assume_offset(UtcOffset::from_hms(1, 2, 3).unwrap());
    let span = Duration::seconds(90) + Duration::nanoseconds(1);
    let earlier = primitive.assume_offset(UtcOffset::from_hms(-1, -2, -3).unwrap());
    let behind = Duration::seconds(-90) + Duration::nanoseconds(-1);
    TimeSample {
        date,
        clock,
        primitive,
        offset,
        span,
        earlier,
        behind,
    }
}

#[cfg(feature = "time")]
fn time_codec_deps() -> String {
    let mut deps = String::new();
    fn push<T: Elm + ElmEncode + ElmDecode>(deps: &mut String) {
        deps.push_str(&T::elm_definition().unwrap());
        deps.push('\n');
        deps.push_str(&T::encoder_definition().unwrap());
        deps.push('\n');
        deps.push_str(&T::decoder_definition().unwrap());
        deps.push('\n');
    }
    push::<time::Date>(&mut deps);
    push::<time::Time>(&mut deps);
    push::<time::PrimitiveDateTime>(&mut deps);
    push::<time::OffsetDateTime>(&mut deps);
    push::<time::Duration>(&mut deps);
    deps
}

#[cfg(feature = "time")]
#[test]
fn time_json_matches_serde_arrays() {
    let sample = sample_time();
    assert_eq!(serde_json::to_string(&sample.date).unwrap(), "[2020,2]");
    assert_eq!(serde_json::to_string(&sample.clock).unwrap(), "[3,4,5,6]");
    assert_eq!(
        serde_json::to_string(&sample.primitive).unwrap(),
        "[2020,2,3,4,5,6]"
    );
    assert_eq!(
        serde_json::to_string(&sample.offset).unwrap(),
        "[2020,2,3,4,5,6,1,2,3]"
    );
    assert_eq!(serde_json::to_string(&sample.span).unwrap(), "[90,1]");
    assert_eq!(time::Duration::elm_type(), "TimeDuration");
    assert!(time::Date::elm_definition().unwrap().contains("year : Int"));
    assert!(
        time::Date::encoder_definition()
            .unwrap()
            .contains("Json.Encode.list")
    );
}

#[cfg(feature = "time")]
#[test]
fn time_types_round_trip_through_elm() {
    super::test_json_with_deps(sample_time(), &time_codec_deps());
}

/// `Duration` and `SystemTime` have no `serde` implementation.
/// These records are an rs2elm shape, checked as text rather than a JSON round trip.
#[test]
fn time_records_are_elm_shapes() {
    assert_eq!(
        Duration::elm_definition().unwrap(),
        "\
type alias Duration =
    { secs : Int
    , nanos : Int
    }
"
    );
    assert!(Duration::encoder_definition().unwrap().contains("\"secs\""));
    assert!(
        Duration::encoder_definition()
            .unwrap()
            .contains("\"nanos\"")
    );
    assert!(Duration::decoder_definition().unwrap().contains("\"secs\""));
    assert!(
        SystemTime::elm_definition()
            .unwrap()
            .contains("secs_since_epoch")
    );
    assert!(
        SystemTime::encoder_definition()
            .unwrap()
            .contains("nanos_since_epoch")
    );
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct BigInts {
    pos: i128,
    neg: i128,
    upos: u128,
    nz: NonZeroI128,
}

#[test]
fn integers_at_the_elm_boundary_round_trip() {
    super::test_json(BigInts {
        pos: 9_007_199_254_740_991,
        neg: -9_007_199_254_740_991,
        upos: 9_007_199_254_740_991,
        nz: NonZeroI128::new(-1).unwrap(),
    });
}

#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct TextCow {
    text: Cow<'static, str>,
}

#[test]
fn cow_str_round_trips_as_a_string() {
    super::test_json(TextCow {
        text: Cow::Borrowed("elm"),
    });
}

#[cfg(feature = "chrono")]
#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct ChronoSample {
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
    date_time: chrono::NaiveDateTime,
    utc: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_values_round_trip_as_strings() {
    let date = chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
    let time = chrono::NaiveTime::from_hms_nano_opt(3, 4, 5, 6).unwrap();
    let date_time = chrono::NaiveDateTime::new(date, time);
    let utc = chrono::TimeZone::from_utc_datetime(&chrono::Utc, &date_time);
    assert_eq!(chrono::NaiveDate::elm_type(), "String");
    assert_eq!(chrono::DateTime::<chrono::Utc>::elm_type(), "String");
    super::test_json(ChronoSample {
        date,
        time,
        date_time,
        utc,
    });
}

#[cfg(feature = "uuid")]
#[derive(Debug, PartialEq, Deserialize, Serialize, Elm, ElmEncode, ElmDecode)]
struct UuidSample {
    id: uuid::Uuid,
}

#[cfg(feature = "uuid")]
#[test]
fn uuid_round_trips_as_a_string() {
    assert_eq!(uuid::Uuid::elm_type(), "String");
    super::test_json(UuidSample {
        id: uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
    });
}
