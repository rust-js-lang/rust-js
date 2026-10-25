// A `loop` that ends an `if`'s branch, whose `else` follows: the loop's
// `break` is its own, and the `else` stays, with no label (ADR 0855). A
// `break 'rows` in a loop inside it is the loop's, by the program's label.

fn walk(items: &[u32], kind: &str, out: &mut Vec<u32>) -> u32 {
    let mut count = 0;
    if kind == "array" {
        for &item in items {
            count += item;
        }
    } else {
        let mut at = 0;
        if kind == "iterable" {
            loop {
                if at >= items.len() {
                    break;
                }
                count += items[at];
                at += 1;
            }
        } else if kind == "object" {
            if items.is_empty() {
                return 7;
            }
            count = 100;
        }
    }
    out.push(count);
    count
}

fn nested(items: &[u32], kind: &str) -> u32 {
    let mut count = 0;
    if kind == "rows" {
        'rows: loop {
            for &item in items {
                if item == 0 {
                    break 'rows;
                }
                match item {
                    1 => break 'rows,
                    2 => count += 2,
                    _ => {}
                }
                count += item;
            }
            if count > 10 {
                break;
            }
        }
    } else if kind == "none" {
        count = 100;
    }
    count + 1
}

fn main() {
    let mut out = Vec::new();
    for kind in ["array", "iterable", "object", "other"] {
        println!("{kind}: {} {}", walk(&[1, 2, 3], kind, &mut out), walk(&[], kind, &mut out));
    }
    println!("{out:?}");
    for kind in ["rows", "none", "other"] {
        println!(
            "{kind}: {} {} {} {}",
            nested(&[3, 4], kind),
            nested(&[2, 0], kind),
            nested(&[5, 1], kind),
            nested(&[9, 9], kind)
        );
    }
}
