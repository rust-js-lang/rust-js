// A test an earlier one decided goes, `if (x != null) { .. } else { if (x
// != null) .. }`, but one that reads through a field is decided only up to
// the next call, which may change the field: `bump` clears `s.v`, so the
// test after it is read again (ADR 0564).
struct Slot {
    v: Option<u32>,
    bumps: u32,
}

fn bump(s: &mut Slot) -> u32 {
    s.bumps += 1;
    s.v = None;
    s.bumps
}

fn read(s: &mut Slot) -> u32 {
    if s.v.is_some() {
        if bump(s) > 3 {
            return 9;
        }
        if s.v.is_some() {
            return 1;
        }
        return 2;
    }
    0
}

// A `match`'s test and a `let else`'s, which no `if` the program wrote
// tests, aren't decided past the call either (#279).
fn matched(s: &mut Slot) -> u32 {
    if s.v.is_none() {
        return 0;
    }
    if bump(s) > 3 {
        return 9;
    }
    match s.v {
        Some(x) => x * 10,
        None => 2,
    }
}

fn let_else(s: &mut Slot) -> u32 {
    let Some(_) = s.v else { return 0 };
    if bump(s) > 3 {
        return 9;
    }
    let Some(x) = s.v else { return 2 };
    x
}

fn main() {
    let mut s = Slot { v: Some(7), bumps: 0 };
    println!("{}", matched(&mut s));
    s.v = Some(7);
    println!("{}", let_else(&mut s));
    println!("{}", read(&mut s));
    println!("{}", read(&mut s));
    s.v = Some(1);
    s.bumps = 3;
    println!("{}", read(&mut s));
}
