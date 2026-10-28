// A `match` of constants that ends a loop's body, a `switch`: a case's
// `continue` of the loop, which the `switch` goes on to anyway, is the
// case's end, `case 0: break;` of the program's `0 => continue`, and none
// where a case ends otherwise (ADR 0629).
fn count(v: &[u32]) -> u32 {
    let mut n = 0;
    for &x in v {
        match x {
            0 => continue,
            1 => n += 1,
            2 => n += 2,
            _ => {
                n += 10;
                continue;
            }
        }
    }
    n
}

fn main() {
    println!("{}", count(&[0, 1, 2, 3, 0, 2]));
    println!("{}", count(&[]));
}
