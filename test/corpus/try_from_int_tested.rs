// An integer's `try_from` whose `Result` is only tested and its `Ok`
// taken is its range test, of the bounds its type can pass, and the
// number: `if (stairsNum < 0) return 0;`, as javascript-algorithms'
// recursiveStaircaseDP writes it (#366). Its number is held where what
// it's of changes before its `Ok` is read; an error read, or the
// `Result` given whole, is std's.

fn stairs(stairs_num: i32) -> f64 {
    let Ok(stairs_num) = usize::try_from(stairs_num) else {
        return 0.0;
    };
    stairs_num as f64 * 2.0
}

fn narrow(n: i64) -> u8 {
    if let Ok(b) = u8::try_from(n) { b } else { 255 }
}

fn widen(n: u32) -> Option<i64> {
    match i64::try_from(n) {
        Ok(wide) => Some(wide + 1),
        Err(_) => None,
    }
}

fn shrink(n: u64) -> i32 {
    match i32::try_from(n) {
        Ok(small) => small,
        Err(_) => -1,
    }
}

fn huge(n: i128) -> u128 {
    let Ok(n) = u128::try_from(n) else {
        return 0;
    };
    n * 2
}

fn said(n: i16) -> String {
    match u8::try_from(n) {
        Ok(b) => format!("byte {b}"),
        Err(e) => format!("not a byte: {e}"),
    }
}

fn whole(n: i32) -> Result<u16, std::num::TryFromIntError> {
    u16::try_from(n)
}

struct Counter {
    at: i32,
}

fn bump(counter: &mut Counter) -> u8 {
    if let Ok(b) = u8::try_from(counter.at) {
        counter.at += 1;
        b
    } else {
        0
    }
}

fn stepped() {
    let mut at = 250i32;
    let mut bytes = Vec::new();
    while at < 260 {
        if let Ok(b) = u8::try_from(at) {
            at += 3;
            bytes.push(b);
        } else {
            at += 4;
        }
    }
    println!("{bytes:?} {at}");
    let mut left = 3i64;
    let mut seen = Vec::new();
    while left > -2 {
        if let Ok(n) = u32::try_from(left * 2) {
            left -= 1;
            seen.push(n);
        } else {
            left -= 1;
        }
    }
    println!("{seen:?} {left}");
}

fn main() {
    for n in [-1, 0, 3] {
        println!("{}", stairs(n));
    }
    for n in [-1i64, 0, 255, 256, i64::MIN, i64::MAX] {
        println!("{}", narrow(n));
    }
    println!("{:?} {:?}", widen(0), widen(u32::MAX));
    for n in [0u64, 2147483647, 2147483648, u64::MAX] {
        println!("{}", shrink(n));
    }
    println!("{} {} {}", huge(-1), huge(1), huge(i128::MAX));
    println!("{} {} {}", said(7), said(-7), said(300));
    println!("{:?} {:?}", whole(65535), whole(65536).is_err());

    stepped();
    let mut counter = Counter { at: 255 };
    println!("{} {} {}", bump(&mut counter), bump(&mut counter), counter.at);
}
