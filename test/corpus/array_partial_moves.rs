// An array some of whose items moved drops what's left, the last first: each
// item left, and each run of them, under the array's flag where it may
// have moved whole, as rustc's `open_drop_for_array` does.
struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pick([a, b, c, d]: [Noisy; 4], i: usize) -> Noisy {
    match i {
        0 => a,
        1 => b,
        2 => c,
        _ => d,
    }
}

fn middle(items: [Noisy; 6]) -> Noisy {
    let [_, _, m, ..] = items;
    println!("took {}", m.0);
    m
}

fn some(items: [Noisy; 5], keep: bool) {
    let [_, b, _, d, _] = items;
    if keep {
        println!("kept {} {}", b.0, d.0);
        std::mem::forget(b);
    }
    println!("end of some");
    drop(d);
}

fn split(items: [Noisy; 4], keep: bool) -> Noisy {
    let [a, rest @ ..] = items;
    println!("rest of {}", rest.len());
    if keep { a } else { Noisy("new") }
}

fn tail(items: [Noisy; 4]) -> [Noisy; 2] {
    let [_, _, rest @ ..] = items;
    rest
}

fn consume(items: [Noisy; 3]) {
    println!("consumed {}", items.len());
}

fn either(items: [Noisy; 3], split: bool) {
    if split {
        let [a, ..] = items;
        println!("first {}", a.0);
    } else {
        consume(items);
    }
    println!("end of either");
}

fn both(items: [Noisy; 3], first: bool) {
    let [a, _, c] = items;
    let mut dropped = 0;
    if first {
        drop(a);
        dropped += 1;
    } else {
        drop(c);
        dropped += 2;
    }
    println!("end of both {}", dropped);
}

fn main() {
    let x = pick([Noisy("a"), Noisy("b"), Noisy("c"), Noisy("d")], 1);
    println!("picked {}", x.0);
    let m = middle([Noisy("1"), Noisy("2"), Noisy("3"), Noisy("4"), Noisy("5"), Noisy("6")]);
    println!("middle {}", m.0);
    some([Noisy("p"), Noisy("q"), Noisy("r"), Noisy("s"), Noisy("t")], true);
    some([Noisy("v"), Noisy("w"), Noisy("x"), Noisy("y"), Noisy("z")], false);
    let a = split([Noisy("e"), Noisy("f"), Noisy("g"), Noisy("h")], true);
    let new = split([Noisy("i"), Noisy("j"), Noisy("k"), Noisy("l")], false);
    println!("split {} {}", a.0, new.0);
    let t = tail([Noisy("m"), Noisy("n"), Noisy("o"), Noisy("u")]);
    println!("tail {}", t.len());
    either([Noisy("E1"), Noisy("E2"), Noisy("E3")], true);
    either([Noisy("F1"), Noisy("F2"), Noisy("F3")], false);
    both([Noisy("G1"), Noisy("G2"), Noisy("G3")], true);
    both([Noisy("H1"), Noisy("H2"), Noisy("H3")], false);
    println!("end");
}
