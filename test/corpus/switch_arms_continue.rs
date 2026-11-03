// A `match` of strings in a loop whose arms `continue`, with more of the
// turn after it: a `switch` whose cases `continue`, as a `match` of the
// same arms that `return` outside a loop is one, and as React DOM's
// clearContainerSparingly writes it (#518). MIR sends each such arm to a
// block whose end is the loop's `continue`, which tidy took for the end
// of an `if`'s branch, the rest its `else`: `if (name === "HTML") { .. }
// else if (name !== "SCRIPT" && ..) { .. }`. A `while let`'s, a `for`'s
// and one whose arm continues after an `if let` alike; one that continues
// at the turn's end; and one of numbers, MIR's `switchInt`, as before.
fn cleared(names: &mut Vec<&'static str>, held: Option<u32>, out: &mut Vec<u32>) {
    while let Some(name) = names.pop() {
        match name {
            "HTML" | "HEAD" | "BODY" => {
                out.push(1);
                continue;
            }
            "SCRIPT" | "STYLE" => continue,
            "LINK" => {
                if let Some(h) = held {
                    out.push(h);
                }
            }
            _ => {}
        }
        out.push(0);
    }
}

fn looped(names: &[&'static str], out: &mut Vec<u32>) {
    for &name in names {
        match name {
            "A" => {
                out.push(1);
                continue;
            }
            "C" => continue,
            "D" => out.push(4),
            _ => {}
        }
        out.push(0);
    }
}

fn at_the_end(names: &[&'static str], out: &mut Vec<u32>) {
    for &name in names {
        out.push(9);
        match name {
            "A" => {
                out.push(1);
                continue;
            }
            "C" => continue,
            "D" => out.push(4),
            _ => out.push(5),
        }
    }
}

fn numbers(values: &[u32], out: &mut Vec<u32>) {
    for &value in values {
        match value {
            1 | 2 => {
                out.push(1);
                continue;
            }
            3 => continue,
            4 => out.push(4),
            _ => {}
        }
        out.push(0);
    }
}

fn main() {
    let mut out = Vec::new();
    let mut names = vec!["X", "LINK", "STYLE", "BODY", "SCRIPT", "HTML", "HEAD", "LINK"];
    cleared(&mut names.clone(), Some(7), &mut out);
    cleared(&mut names, None, &mut out);
    println!("{:?}", out);
    let mut out = Vec::new();
    looped(&["A", "B", "C", "D", "E"], &mut out);
    println!("{:?}", out);
    let mut out = Vec::new();
    at_the_end(&["A", "B", "C", "D", "E"], &mut out);
    println!("{:?}", out);
    let mut out = Vec::new();
    numbers(&[1, 2, 3, 4, 5], &mut out);
    println!("{:?}", out);
}
