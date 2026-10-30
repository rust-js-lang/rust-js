//@ library-differs
// A pattern of a trait's constant, `<u8 as Level>::HIGH`, of a type: its
// value, which needs no Self to say (ADR 0590). A crate's own constant is
// still its name.
trait Level {
    const HIGH: u8;
    const LOW: u8;
}

struct Byte;

impl Level for Byte {
    const HIGH: u8 = 200;
    const LOW: u8 = 10;
}

impl Level for u8 {
    const HIGH: u8 = 250;
    const LOW: u8 = 5;
}

const MIDDLE: u8 = 100;

fn name(n: u8) -> &'static str {
    match n {
        <u8 as Level>::HIGH => "u8 high",
        <Byte as Level>::HIGH => "byte high",
        <Byte as Level>::LOW | <u8 as Level>::LOW => "low",
        MIDDLE => "middle",
        _ => "other",
    }
}

fn main() {
    for n in [250, 200, 10, 5, 100, 7] {
        println!("{n}: {}", name(n));
    }
    let n: u8 = 200;
    if let <Byte as Level>::HIGH = n {
        println!("matched by let");
    }
}
