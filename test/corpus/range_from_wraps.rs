// An `a..`'s `next()` past its type's last integer wraps to its first, as
// release Rust's does, and a `char`'s steps by its code point, past the
// surrogates, whether it's stepped as it is or as an iterator (#414).

fn main() {
    let mut letters = 'a'..;
    println!("char {:?} {:?}", letters.next(), letters.next());
    let mut surrogates = '\u{D7FF}'..;
    println!("past surrogates {:?} {:?}", surrogates.next(), surrogates.next());

    // Past the last `char`, std's panic, before it gives one.
    let last = std::panic::catch_unwind(|| {
        let mut r = char::MAX..;
        r.next()
    });
    println!("past the last char: {}", last.is_err());

    let mut bytes = 255u8..;
    println!("u8 {:?} {:?}", bytes.next(), bytes.next());
    let mut small = i8::MAX..;
    println!("i8 {:?} {:?}", small.next(), small.next());
    let mut wide = i32::MAX..;
    println!("i32 {:?} {:?}", wide.next(), wide.next());
    let mut count = u16::MAX..;
    println!("u16 {:?} {:?}", count.next(), count.next());
    let mut big = i64::MAX..;
    println!("i64 {:?} {:?}", big.next(), big.next());
    let mut huge = u128::MAX..;
    println!("u128 {:?} {:?}", huge.next(), huge.next());

    let firsts: Vec<u8> = (254u8..).take(4).collect();
    println!("{firsts:?}");
    let stepped: Vec<u8> = (250u8..).step_by(3).take(4).collect();
    println!("{stepped:?}");
    let skipped: Vec<u8> = (250u8..).skip(3).take(5).collect();
    println!("{skipped:?}");
    let longs: Vec<i64> = (i64::MAX - 1..).take(3).collect();
    println!("{longs:?}");
    let chars: String = ('x'..).take(4).collect();
    println!("{chars}");

    let mut seen = 0;
    for b in 250u8.. {
        seen += 1;
        if seen > 8 {
            break;
        }
        print!("{b} ");
    }
    println!();
    // A `char`'s, stepped past the surrogates.
    for c in '\u{D7FE}'.. {
        if c > '\u{E001}' {
            break;
        }
        print!("{:x} ", c as u32);
    }
    println!();
}
