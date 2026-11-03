// Each comment of a `match`'s arms is kept where it's written (#574): a
// block comment on a `_` pattern, `_ /* "{" */ => {`, of no `default:`, the
// `match` gone or its arm an `else`, before the arm's first code, and of one
// that panics on its line, `_ /* "x" */ => panic!(..)`, after its
// `default:`; a comment above an arm of the unit value, `1 => (),`, above
// its `case`, as above `1 => {}`; and one above a guarded arm that does
// nothing, `1 if skip => {}`, whose test the next arm's is, before that
// test, in the `else`'s block.
pub fn first(tag: u8, row: &str) -> Vec<String> {
    let mut log = Vec::new();
    match tag {
        // Anything else first.
        _ /* "{" */ => {
            log.push(row.to_string());
            log.push("again".to_string());
        }
    }
    log
}

pub fn middle(tag: u8, log: &mut Vec<u8>) {
    match tag {
        82 /* "R" */ => {
            log.push(1);
        }
        _ /* "{" "}" */ => {
            log.push(2);
            log.push(3);
        }
    }
    log.push(4);
}

pub fn thrown(tag: u8) -> u8 {
    match tag {
        1 => println!("one"),
        2 => println!("two"),
        3 => println!("three"),
        // Never.
        _ /* "x" */ => panic!("no {tag}"),
    }
    tag
}

pub fn unit(n: u8, log: &mut Vec<u8>) {
    match n {
        0 => log.push(0),
        // One is fine.
        1 => (),
        _ => log.push(n),
    }
}

pub fn unit_str(n: &str, log: &mut Vec<u8>) {
    match n {
        "a" => log.push(0),
        // One is fine.
        "b" => (),
        _ => log.push(9),
    }
}

pub fn guarded(n: u8, skip: bool, log: &mut Vec<u8>) {
    match n {
        0 => log.push(0),
        // One is fine when skipped.
        1 if skip => {}
        _ => log.push(n),
    }
}

fn main() {
    println!("{:?}", first(7, "row"));
    let mut log = Vec::new();
    middle(82, &mut log);
    middle(0, &mut log);
    for n in [0, 1, 2] {
        unit(n, &mut log);
        guarded(n, true, &mut log);
        guarded(n, false, &mut log);
    }
    for s in ["a", "b", "c"] {
        unit_str(s, &mut log);
    }
    println!("{log:?}");
    for tag in [1, 2, 3] {
        println!("{}", thrown(tag));
    }
    let caught = std::panic::catch_unwind(|| thrown(9));
    println!("{}", caught.is_err());
}
