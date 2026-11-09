// A comment after a literal of each kind, as libm's `k_tan.rs` writes one
// after each coefficient: the comment goes after the whole literal, never
// inside it, where `3.3e /* .. */-01` is no JS rust-js can read back
// (#1117).
#[rustfmt::skip]
static FLOATS: [f64; 4] = [
    3.33333333333334091986e-01,  /* 3FD55555, 55555563 */
    1.0E+10,  /* an upper exponent with its sign */
    2.5e3,  /* no sign */
    0.5,  /* none */
];

#[rustfmt::skip]
static INTS: [u32; 3] = [
    0xff_ff,  /* hex with separators */
    0b1010,  /* binary */
    1_000_000,  /* decimal with separators */
];

#[rustfmt::skip]
static WIDE: [u64; 2] = [
    18_446_744_073_709_551_615,  /* a BigInt in JS */
    0x1e5,  /* hex whose digits hold an `e` */
];

#[rustfmt::skip]
static TEXTS: [&str; 3] = [
    "a\"b",  /* an escaped quote */
    "it's",  /* a quote of the other kind */
    "{braces}",  /* braces */
];

fn main() {
    let tiny = 5.0e-3; /* a local's */
    let flag = true; /* a boolean */
    let letter = 'x'; /* a char */
    println!("{FLOATS:?} {INTS:?} {WIDE:?}");
    println!("{TEXTS:?} {tiny} {flag} {letter}");
}
