// A `continue` the program wrote in a `match`'s arm, and what it wrote
// after the `match`: the arm's `case` ends in its `continue`, and what
// follows is after the `switch`, once, `switch (x) { case 0: ..;
// continue; case 1: ..; } if (x > 5) { ..; continue; }`, not each case's
// copy of it, `case 1: ..; if (x > 5) { .. } break; default: if (x > 5) {
// .. }` (kj-switch).

fn numbers(xs: &[i32]) -> Vec<i32> {
    let mut out = Vec::new();
    for &x in xs {
        match x {
            0 => {
                out.push(1);
                continue;
            }
            1 => out.push(2),
            _ => {}
        }
        if x > 5 {
            out.push(x);
            continue;
        }
        out.push(-x);
    }
    out
}

fn bytes(text: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for b in text.bytes() {
        match b {
            b' ' => continue,
            b'a' | b'e' => out.push(b'!'),
            b'z' => {
                out.push(b'?');
                continue;
            }
            _ => {}
        }
        if b > b'm' {
            out.push(b.to_ascii_uppercase());
            continue;
        }
        out.push(b);
    }
    out
}

fn main() {
    println!("{:?}", numbers(&[0, 1, 2, 7, 1, 0]));
    println!("{:?}", bytes("a zebra eats"));
}
