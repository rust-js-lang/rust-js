use std::error::Error;
use std::fmt::{self, Write};
use std::num::NonZeroU32;

/// Whether each window of `bytes` has a byte that isn't 0: of 64 random
/// bytes, all 0 is a chance of 2^-512. Each window is 64 bytes, the last
/// one ending at the end, where `chunks` gave a short one: of 65,537
/// bytes a window of 1, which is 0 once in 256 runs.
fn filled(bytes: &[u8]) -> bool {
    let from = |at: usize| at.min(bytes.len().saturating_sub(64));
    (0..bytes.len())
        .step_by(64)
        .all(|at| bytes[from(at)..].iter().take(64).any(|&b| b != 0))
}

/// What a fill of each length gives: its bytes filled, across the edge of
/// one `getRandomValues`, 65,536 bytes; and a part of a slice filled, the
/// rest left as it was.
fn fills(name: &str, fill: impl Fn(&mut [u8]) -> bool, out: &mut String) {
    for len in [0, 1, 63, 64, 65_535, 65_536, 65_537, 70_000] {
        let mut bytes = vec![0u8; len];
        let ok = fill(&mut bytes);
        let whole = len < 64 || filled(&bytes);
        let last = len < 64 || filled(&bytes[len - 64..]);
        writeln!(out, "{name} {len}: ok {ok} filled {whole} last {last}").unwrap();
    }
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    let both = fill(&mut a) && fill(&mut b);
    writeln!(out, "{name} two: {both} differ {}", a != b).unwrap();
    let mut part = [0u8; 200];
    let ok = fill(&mut part[50..150]);
    let rest = part[..50].iter().chain(&part[150..]).all(|&b| b == 0);
    writeln!(out, "{name} part: {ok} filled {} rest {rest}", filled(&part[50..150])).unwrap();
}

/// Each piece a writer is given, as `write!` gives them.
struct Pieces(Vec<String>);

impl Write for Pieces {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

/// How an error shows: `{}`, `{:?}`, `{:#?}`, padded, and the pieces a
/// writer is given of each; and as a `Box<dyn Error + Send + Sync>`.
fn shown<E: Error + Send + Sync + 'static>(name: &str, e: E, out: &mut String) {
    writeln!(out, "{name}: {e} | {e:?} | [{e:>48}] | {e:12?}").unwrap();
    writeln!(out, "{name} pretty: {e:#?}").unwrap();
    let mut pieces = Pieces(Vec::new());
    write!(pieces, "{e}|{e:?}|{e:#?}").unwrap();
    writeln!(out, "{name} pieces: {:?}", pieces.0).unwrap();
    let boxed: Box<dyn Error + Send + Sync> = Box::new(e);
    let source = boxed.source().is_none();
    let text = boxed.to_string();
    writeln!(out, "{name} boxed: {boxed} | {boxed:?} | {text} | source none {source}").unwrap();
}

/// What a `Box<dyn Error + Send + Sync>` of one finds downcast to each
/// version's `Error`, of which only its own is it.
fn downcasts(boxed: Box<dyn Error + Send + Sync>) -> String {
    let v01 = boxed.downcast_ref::<getrandom01::Error>().map(|e| e.code());
    let v02 = boxed.downcast_ref::<getrandom02::Error>().map(|e| e.code());
    let v03 = boxed.downcast_ref::<getrandom03::Error>().map(|e| e.to_string());
    let v04 = boxed.downcast_ref::<getrandom04::Error>().is_some();
    format!("{v01:?} {v02:?} {v03:?} {v04}")
}

fn v01(out: &mut String) {
    use getrandom01::Error;
    fills("0.1", |dest| getrandom01::getrandom(dest).is_ok(), out);
    #[allow(deprecated)]
    let constants = [Error::UNKNOWN, Error::UNAVAILABLE];
    for e in constants {
        writeln!(out, "0.1 constant: {} {:?}", e.code(), e.raw_os_error()).unwrap();
        shown("0.1 constant", e, out);
    }
    let codes = [
        Error::INTERNAL_START,
        Error::INTERNAL_START + 2,
        Error::INTERNAL_START + 11,
        Error::INTERNAL_START + 12,
        Error::CUSTOM_START + 3,
        1,
        2,
        13,
        9_999,
    ];
    for code in codes {
        let e = Error::from(NonZeroU32::new(code).unwrap());
        let into: Error = NonZeroU32::new(code).unwrap().into();
        writeln!(out, "0.1 {code}: {} {:?} {}", e.code(), e.raw_os_error(), into.code()).unwrap();
        shown(&format!("0.1 {code}"), e, out);
    }
}

fn v02(out: &mut String) {
    use getrandom02::Error;
    fills("0.2", |dest| getrandom02::getrandom(dest).is_ok(), out);
    let mut uninit = [std::mem::MaybeUninit::<u8>::uninit(); 100];
    let given = getrandom02::getrandom_uninit(&mut uninit).map(|bytes| (bytes.len(), filled(bytes)));
    writeln!(out, "0.2 uninit: {given:?}").unwrap();
    let constants = [
        Error::UNSUPPORTED,
        Error::ERRNO_NOT_POSITIVE,
        Error::UNEXPECTED,
        Error::WEB_CRYPTO,
        Error::NODE_ES_MODULE,
    ];
    for e in constants {
        writeln!(out, "0.2 constant: {} {:?}", e.code(), e.raw_os_error()).unwrap();
        shown("0.2 constant", e, out);
    }
    let codes = [
        Error::INTERNAL_START + 3,
        Error::INTERNAL_START + 9,
        Error::INTERNAL_START + 13,
        Error::CUSTOM_START,
        1,
        2,
        9_999,
    ];
    for code in codes {
        let e = Error::from(NonZeroU32::new(code).unwrap());
        writeln!(out, "0.2 {code}: {} {:?}", e.code(), e.raw_os_error()).unwrap();
        shown(&format!("0.2 {code}"), e, out);
    }
}

fn v03(out: &mut String) {
    use getrandom03::Error;
    fills("0.3", |dest| getrandom03::fill(dest).is_ok(), out);
    let mut uninit = [std::mem::MaybeUninit::<u8>::uninit(); 100];
    let given = getrandom03::fill_uninit(&mut uninit).map(|bytes| (bytes.len(), filled(bytes)));
    writeln!(out, "0.3 uninit: {given:?}").unwrap();
    let u32s: Vec<u32> = (0..8).map(|_| getrandom03::u32().unwrap()).collect();
    let u64s: Vec<u64> = (0..8).map(|_| getrandom03::u64().unwrap()).collect();
    let varied = u32s.iter().any(|&n| n != u32s[0]) && u64s.iter().any(|&n| n != u64s[0]);
    let wide = u64s.iter().any(|&n| n > u64::from(u32::MAX));
    writeln!(out, "0.3 numbers: varied {varied} wide {wide}").unwrap();
    for e in [
        Error::UNSUPPORTED,
        Error::ERRNO_NOT_POSITIVE,
        Error::UNEXPECTED,
        Error::new_custom(0),
        Error::new_custom(7),
    ] {
        writeln!(out, "0.3 constant: {:?}", e.raw_os_error()).unwrap();
        shown("0.3", e, out);
    }
}

fn v04(out: &mut String) {
    use getrandom04::Error;
    fills("0.4", |dest| getrandom04::fill(dest).is_ok(), out);
    let mut uninit = [std::mem::MaybeUninit::<u8>::uninit(); 100];
    let given = getrandom04::fill_uninit(&mut uninit).map(|bytes| (bytes.len(), filled(bytes)));
    writeln!(out, "0.4 uninit: {given:?}").unwrap();
    let u64s: Vec<u64> = (0..8).map(|_| getrandom04::u64().unwrap()).collect();
    writeln!(out, "0.4 numbers: varied {}", u64s.iter().any(|&n| n != u64s[0])).unwrap();
    for e in [Error::UNSUPPORTED, Error::UNEXPECTED, Error::new_custom(3)] {
        writeln!(out, "0.4 constant: {:?}", e.raw_os_error()).unwrap();
        shown("0.4", e, out);
    }
}

/// rand_core 0.5's, which itertools' quickcheck reaches: `OsRng`'s fill,
/// `getrandom(dest)?` into its `Error`, and its `From<getrandom::Error>`,
/// a `Box<dyn Error + Send + Sync>` of it, asked its code, which it finds
/// by a downcast to its own type, and none.
fn rand_core_05(out: &mut String) {
    writeln!(
        out,
        "downcasts: {}",
        downcasts(Box::new(getrandom01::Error::from(NonZeroU32::new(5).unwrap())))
    )
    .unwrap();
    writeln!(
        out,
        "downcasts: {}",
        downcasts(Box::new(getrandom02::Error::UNEXPECTED))
    )
    .unwrap();
    writeln!(
        out,
        "downcasts: {}",
        downcasts(Box::new(getrandom03::Error::UNEXPECTED))
    )
    .unwrap();
    writeln!(
        out,
        "downcasts: {}",
        downcasts(Box::new(getrandom04::Error::UNEXPECTED))
    )
    .unwrap();
    use rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 64];
    let ok = OsRng.try_fill_bytes(&mut bytes).is_ok();
    let n = OsRng.next_u64();
    writeln!(out, "rand_core fill: {ok} {} {}", filled(&bytes), n != OsRng.next_u64()).unwrap();
    #[allow(deprecated)]
    let e = rand_core::Error::from(getrandom01::Error::UNAVAILABLE);
    writeln!(
        out,
        "rand_core error: {e} | {e:?} | {:?} {:?} | source none {}",
        e.code(),
        e.raw_os_error(),
        e.source().is_none()
    )
    .unwrap();
    let inner = e.take_inner();
    writeln!(out, "rand_core inner: {inner} | {inner:?}").unwrap();
}

pub fn report() -> String {
    let mut out = String::new();
    v01(&mut out);
    v02(&mut out);
    v03(&mut out);
    v04(&mut out);
    rand_core_05(&mut out);
    out
}

pub fn main() {
    print!("{}", report());
}
