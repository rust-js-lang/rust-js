// A function JS calls, generic, whose call of the crate's own generic
// function is of a copy that turns out alike it, `parseInternalUnboxed`
// then `parseInternal`, its result named for the call (ADR 0569): the
// local is named apart, so the call reads the function, not the local
// before it's made, as chrono's `format::parse::parse` threw (#467).
use std::borrow::Borrow;

fn parse_internal<'a, I, B>(s: &'a str, items: I) -> Result<&'a str, (&'a str, u32)>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    let mut rest = s;
    for item in items {
        match rest.strip_prefix(*item.borrow() as char) {
            Some(r) => rest = r,
            None => return Err((rest, 1)),
        }
    }
    Ok(rest)
}

pub fn parse<I, B>(s: &str, items: I) -> Result<(), u32>
where
    I: Iterator<Item = B>,
    B: Borrow<u8>,
{
    match parse_internal(s, items) {
        Ok("") => Ok(()),
        Ok(_) => Err(2),
        Err((_, e)) => Err(e),
    }
}

fn main() {
    let items = [b'x', b'y'];
    println!("{:?}", parse("xy", items.iter()));
    println!("{:?}", parse("xyz", items.iter()));
    println!("{:?}", parse("y", items.iter()));
}
