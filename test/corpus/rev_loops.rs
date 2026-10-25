// A `for` over a slice's items, or a range, from its end, `rev()`: a JS
// `for` that counts down, `for (let i = v.length - 1; i >= 0; i--)`, as
// React's `getComponentNameFromFiber` reads its debug info, no copy of the
// array made (ADR 0904).

fn last_long(names: &[String]) -> Option<&String> {
    for name in names.iter().rev() {
        if name.len() > 3 {
            return Some(name);
        }
    }
    None
}

fn weighted(v: &[u32]) -> u32 {
    let mut total = 0;
    let mut weight = 1;
    for x in v.iter().rev() {
        if *x == 0 {
            continue;
        }
        if *x > 100 {
            break;
        }
        total += x * weight;
        weight += 1;
    }
    total
}

fn pairs(v: &[(u8, char)]) -> String {
    let mut out = String::new();
    for &(n, c) in v.iter().rev() {
        for _ in 0..n {
            out.push(c);
        }
    }
    out
}

fn owned(v: Vec<String>) -> String {
    let mut out = String::new();
    'words: for word in v.into_iter().rev() {
        for c in word.chars() {
            if c == '!' {
                break 'words;
            }
        }
        out.push_str(&word);
    }
    out
}

fn counted(from: i32, to: i32) -> Vec<i32> {
    let mut seen = Vec::new();
    for i in (from..to).rev() {
        seen.push(i);
    }
    for i in (from..=to).rev() {
        seen.push(i * 10);
    }
    let mut n = 0;
    for _ in (0..to.max(0) as usize).rev() {
        n += 1;
    }
    seen.push(n);
    seen
}

/// A range's start the body changes, which Rust read once, as it made the
/// range; and a range of `char`s, which JS's `--` doesn't step.
fn moving(n: i32) -> (i32, String) {
    let mut lo = 0;
    let mut count = 0;
    for i in (lo..n).rev() {
        lo += 2;
        count += i;
    }
    let mut letters = String::new();
    for c in ('a'..='e').rev() {
        letters.push(c);
    }
    (count + lo, letters)
}

fn main() {
    let names: Vec<String> = ["a", "react", "dom", "fiber", "x"].iter().map(|s| s.to_string()).collect();
    println!("{:?} {:?}", last_long(&names), last_long(&names[..1]));
    println!("{} {} {}", weighted(&[1, 2, 0, 3]), weighted(&[5, 200, 7, 0]), weighted(&[]));
    println!("{:?}", pairs(&[(2, 'a'), (0, 'b'), (3, 'c')]));
    let words: Vec<String> = ["one", "two!", "three"].iter().map(|s| s.to_string()).collect();
    println!("{:?} {:?}", owned(words), owned(Vec::new()));
    println!("{:?} {:?} {:?}", counted(-2, 3), counted(4, 4), counted(5, 2));
    println!("{:?} {:?}", moving(5), moving(0));
}
