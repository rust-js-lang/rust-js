//! A library whose derived `Debug` of more than five fields makes `&dyn
//! Debug`s of them, which only std's `debug_struct_fields_finish` is given
//! and which never leave it, as chrono's do: it makes no `dyn` a consumer
//! may write, so a consumer's `dyn` written into a writer isn't refused
//! (#1688).
#[derive(Debug)]
struct Wide {
    a: i32,
    b: &'static str,
    c: char,
    d: Option<u8>,
    e: bool,
    f: u64,
}

pub fn described(n: i32) -> String {
    let wide = Wide {
        a: n,
        b: "b\"",
        c: 'c',
        d: Some(4),
        e: true,
        f: 6,
    };
    format!("{wide:?}")
}
