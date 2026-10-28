// A loop left by a `break` of a choice, where its test is `a || b`: the
// break's value is `if c { 1 } else { 2 }`, each arm the loop's value.
// Found by a generated program (seed 40074), which rust-js's tidying of
// the loop's jumps never finished compiling: it turned `if (u) { return
// 1; } return 2;` into `if (!u) { return 2; } return 1;`, and back. And
// a loop whose body ends in a guard that returns, then a `return`: an exit
// each way already, as written.

fn id<T>(x: T) -> T {
    x
}

fn note<T>(tag: u32, x: T) -> T {
    println!("note {tag}");
    x
}

fn choose(c: u32, first: bool) -> i16 {
    loop {
        if c > 3 || id(false) {
            break if id(first) { 1 } else { 2 };
        }
        return -1;
    }
}

fn counted(limit: u32, stop: bool) -> i16 {
    let mut c = 0u32;
    loop {
        c += 1;
        if c > id(limit) || note(c, stop) {
            break if c % 2 == 0 { 10 } else { 20 };
        }
    }
}

fn parity(limit: u32, stop: bool) -> i16 {
    let mut c = 0u32;
    loop {
        c += 1;
        if c <= limit && !note(c, stop) {
            continue;
        }
        if c.is_multiple_of(2) {
            return 10;
        }
        return 20;
    }
}

fn main() {
    println!("{} {} {}", choose(5, true), choose(5, false), choose(1, true));
    println!("{} {}", counted(3, false), counted(4, true));
    println!("{} {}", parity(3, false), parity(4, true));
}
