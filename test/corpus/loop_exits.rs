// A `while let` whose loop returns where it ends, as a person writes its
// exits: `if (part == null) { return; }` first, a `?`'s `if (!value) {
// return; }` too, not the rest nested in `if (value) { .. }`; and one that
// goes on, `if (!c) { break; }`, its `return` after the loop. Not where the
// `return` reads what the loop declares, nor where something else leaves
// the loop, which would reach that `return` too, nor where what's before
// the exit doesn't leave, `once`'s, nor where it declares a name the body
// has, `shadowed`'s.
use std::collections::HashMap;

enum Entry {
    Dir(HashMap<String, Entry>),
    File(String),
}

fn text_in(root: &HashMap<String, Entry>, path: &str) -> Option<String> {
    let mut dir = root;
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        match (dir.get(part)?, parts.peek().is_some()) {
            (Entry::Dir(inner), true) => dir = inner,
            (Entry::File(text), false) => return Some(text.clone()),
            _ => return None,
        }
    }
    None
}

fn first_big(v: &mut Vec<u32>) -> u32 {
    while let Some(x) = v.pop() {
        let y = x * 2;
        if y < 10 {
            continue;
        }
        return y;
    }
    0
}

fn small_sum(v: &mut Vec<u32>, flag: bool) -> u32 {
    let mut total = 0;
    if flag {
        while let Some(x) = v.pop() {
            if x < 50 {
                total += x;
                continue;
            }
            return 7;
        }
    }
    total + 1
}

fn once(v: &mut Vec<u32>) -> u32 {
    let mut n = 0;
    while let Some(x) = v.pop() {
        if x == 0 {
            continue;
        }
        if x > 5 {
            n += 1;
        }
        return n;
    }
    n + 100
}

fn shadowed(v: &mut Vec<u32>) -> u32 {
    let n = 0;
    while let Some(x) = v.pop() {
        let y = x + 1;
        if y == 1 {
            continue;
        }
        if y > 2 {
            let y = 5;
            return n + y;
        }
        return n + y;
    }
    n
}

fn first_large(v: &mut Vec<u32>) -> Option<u32> {
    let mut n = 0;
    loop {
        let t = v.pop();
        match t {
            Some(x) if x < 5 => {
                n += x;
                continue;
            }
            _ => return t.map(|x| x + n),
        }
    }
}

fn stopped(v: &mut Vec<u32>) -> u32 {
    let mut n = 0;
    loop {
        let t = v.pop();
        match t {
            Some(0) => break,
            Some(x) if x < 5 => {
                n += x;
                continue;
            }
            _ => return n,
        }
    }
    n + 100
}

fn main() {
    let mut inner = HashMap::new();
    inner.insert("b.txt".to_string(), Entry::File("hello".to_string()));
    let mut root = HashMap::new();
    root.insert("a".to_string(), Entry::Dir(inner));
    root.insert("c.txt".to_string(), Entry::File("top".to_string()));
    for path in ["a/b.txt", "c.txt", "a", "a/x", "c.txt/y", ""] {
        println!("{path}: {:?}", text_in(&root, path));
    }
    println!("{} {}", first_big(&mut vec![9, 2, 3]), first_big(&mut vec![1, 2]));
    println!("{} {} {}", once(&mut vec![9, 9]), once(&mut vec![1, 0]), once(&mut vec![]));
    println!("{} {} {}", shadowed(&mut vec![1, 0]), shadowed(&mut vec![3, 0]), shadowed(&mut vec![]));
    println!("{:?} {:?}", first_large(&mut vec![9, 1, 2]), first_large(&mut vec![1]));
    println!("{} {} {}", stopped(&mut vec![0, 1, 2]), stopped(&mut vec![9, 1]), stopped(&mut vec![]));
    println!("{} {} {}", small_sum(&mut vec![1, 2], true), small_sum(&mut vec![60, 1], true), small_sum(&mut vec![], false));
}
