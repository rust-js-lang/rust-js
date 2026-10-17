// A temporary of a variable that changes is the variable where it's read
// before it changes, and its own value where it's read after.

fn pair(a: i32, b: i32) -> String {
    format!("({}, {})", a, b)
}

fn joined(a: String, b: String) -> String {
    format!("{}+{}", a, b)
}

fn kind(word: &str) -> u8 {
    match word {
        "let" => 1,
        "fn" => 2,
        _ => 0,
    }
}

fn main() {
    // Read before it changes, then changed.
    let mut word = String::from("let");
    let k = match word.as_str() {
        "let" => 1,
        _ => 2,
    };
    word.push_str("ter");
    println!("{} {} {}", k, word, kind(&word));

    // Read, then changed in a later argument: the value it had.
    let mut n = 5;
    println!("{}", pair(n, {
        n += 1;
        n
    }));
    let mut s = String::from("a");
    println!("{}", joined(s.clone(), {
        s.push('b');
        s.clone()
    }));
    // Changed, then read again: the new value.
    let mut m = 1;
    for _ in 0..3 {
        let before = m;
        m = m * 2 + before;
        println!("{} {}", before, m);
    }
    // A closure made before it changes, called after.
    let mut count = 10;
    let shown = {
        let at = count;
        move || at * 2
    };
    count += 1;
    println!("{} {}", shown(), count);
}
