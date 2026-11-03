// A block with statements in an `if`'s condition, `check(f) || {
// flags.set(f & !32); false }`, as alien-signals' computedOper writes
// it: the block's statements and its value one comma expression in the
// condition, `check(f) || ((flags.value = ..), false)`, as upstream's JS
// writes it, not labeled blocks the Rust doesn't have (#523). Each part
// runs where Rust runs it and only on its way: a call logs, a field set,
// two statements, a block whose value is a test, one in an `&&`, and the
// `else if` after it. A field read before it is read before it still, and
// a local's set, which sets a binding, is no part of one yet.
use std::cell::{Cell, RefCell};

fn check(log: &RefCell<Vec<String>>, n: u32) -> bool {
    log.borrow_mut().push(format!("check {n}"));
    n > 40
}

fn step(log: &RefCell<Vec<String>>, flags: &Cell<u32>, value: &Cell<u32>) -> u32 {
    let f = flags.get();
    if f & 16 != 0
        || (f & 32 != 0
            && (check(log, f) || {
                flags.set(f & !32);
                false
            }))
    {
        value.set(value.get() + 10);
    } else if f == 0 {
        flags.set(1);
        value.set(value.get() + 1);
    }
    value.get()
}

fn logged(log: &RefCell<Vec<String>>, flags: &Cell<u32>, f: u32) -> bool {
    if f & 1 != 0
        && (check(log, f) || {
            log.borrow_mut().push(format!("unset {f}"));
            flags.set(f & !1);
            check(log, f + 1)
        })
    {
        log.borrow_mut().push("taken".to_string());
        return true;
    }
    log.borrow_mut().push("past".to_string());
    false
}

fn guarded(log: &RefCell<Vec<String>>, flags: &Cell<u32>, f: u32) -> u32 {
    let mut n = 0;
    if f > 3
        && ({
            flags.set(flags.get() + 1);
            true
        } && check(log, f))
    {
        n += 1;
    } else {
        n += 2;
    }
    n
}

fn read_before(log: &RefCell<Vec<String>>, flags: &Cell<u32>, f: u32) -> u32 {
    let before = flags.get();
    if f & 1 != 0
        || (f & 2 != 0
            && (check(log, f) || {
                flags.set(f + 100);
                false
            }))
    {
        return 0;
    }
    before + flags.get()
}

fn local(log: &RefCell<Vec<String>>, f: u32) -> u32 {
    let mut seen = 1;
    if f & 1 != 0
        || (f & 2 != 0
            && (check(log, f) || {
                seen = 5;
                false
            }))
    {
        seen += 10;
    }
    seen
}

fn marked(log: &RefCell<Vec<String>>, flags: &Cell<u32>, f: u32) -> u32 {
    if f & 1 != 0
        || (f & 2 != 0
            && (check(log, f) || {
                flags.set(f | 64);
                true
            }))
    {
        return flags.get();
    }
    0
}

struct Counter {
    n: u32,
}

fn through(log: &RefCell<Vec<String>>, counter: &mut Counter, f: u32) -> u32 {
    let n = &mut counter.n;
    if f & 1 != 0
        || (f & 2 != 0
            && (check(log, f) || {
                *n = 9;
                false
            }))
    {
        return 0;
    }
    *n + 1
}

fn main() {
    let log = RefCell::new(Vec::new());
    for f in [0, 16, 32, 33, 48, 50] {
        let flags = Cell::new(f);
        let value = Cell::new(0);
        let v = step(&log, &flags, &value);
        println!("step {f}: {v} {} {:?}", flags.get(), log.borrow());
        log.borrow_mut().clear();
    }
    for f in [0, 1, 3, 41, 43] {
        let flags = Cell::new(f);
        let taken = logged(&log, &flags, f);
        println!("logged {f}: {taken} {} {:?}", flags.get(), log.borrow());
        log.borrow_mut().clear();
    }
    for f in [2, 5, 50] {
        let flags = Cell::new(f);
        let n = guarded(&log, &flags, f);
        println!("guarded {f}: {n} {} {:?}", flags.get(), log.borrow());
        log.borrow_mut().clear();
    }
    for f in [1, 2, 4, 50] {
        let flags = Cell::new(7);
        println!("read before {f}: {}", read_before(&log, &flags, f));
        println!("local {f}: {}", local(&log, f));
        println!("marked {f}: {}", marked(&log, &flags, f));
        let mut counter = Counter { n: 3 };
        println!("through {f}: {} {}", through(&log, &mut counter, f), counter.n);
    }
}
