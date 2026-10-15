//@ compile-fail: rust-js does not support addressable memory yet: raw addresses of statics
// A static's raw address has no JS value (ADR 0096).
static LIMIT: u32 = 7;

fn main() {
    let limit = &raw const LIMIT;
    println!("{}", unsafe { *limit });
}
