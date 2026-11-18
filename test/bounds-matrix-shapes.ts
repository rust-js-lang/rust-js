// The bounds matrix's shape family (ADR 1137's audit), its programs
// `shape_<name>`, which `programs()` of `bounds-matrix.ts` takes with the
// collection holders' and the value family's.
//
// The shape family: each rule the audit's ablation found proving corpus
// sites but no matrix row, its own shape, a control its rule proves and
// each write that would break the rule run against native (rustc-tests.yml
// `ablation`'s gaps). Where nothing between can break the proof, a
// quotient's or a live slice's length sum, the control alone holds the rule.

const shapeDecls = `use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}
`;

/** Each shape: its functions, and the calls `main` checks of them. */
const shapes: Record<string, { fns: string; calls: string[] }> = {
  // `x != 0` narrows `x` past its least, so `x - 1` never wraps: an
  // iterator's `next`. Not past a write of 0 between.
  unequal: {
    fns: `struct Count {
    n: u32,
}

fn next(c: &mut Count) -> u32 {
    if c.n != 0 {
        c.n -= 1;
        c.n
    } else {
        99
    }
}

fn zeroed(c: &mut Count) -> u32 {
    if c.n != 0 {
        c.n = 0;
        c.n -= 1;
        c.n
    } else {
        99
    }
}

fn reset(c: &mut Count) {
    c.n = 0;
}

fn reset_between(c: &mut Count) -> u32 {
    if c.n != 0 {
        reset(c);
        c.n -= 1;
        c.n
    } else {
        99
    }
}

fn through_a_cell(n: &Cell<u32>, cut: &dyn Fn()) -> u32 {
    if n.get() != 0 {
        cut();
        n.get() - 1
    } else {
        99
    }
}
`,
    calls: [
      `check("next", || next(&mut Count { n: 3 }))`,
      `check("zeroed", || zeroed(&mut Count { n: 3 }))`,
      `check("reset between", || reset_between(&mut Count { n: 3 }))`,
      `check("through a cell", || { let n = Cell::new(3); through_a_cell(&n, &|| n.set(0)) })`,
    ],
  },
  // Two lengths summed never wrap: no allocation holds `usize::MAX`.
  lengths: {
    fns: `fn summed(a: &[u32], b: &[u32]) -> usize {
    a.len() + b.len()
}

fn text_summed(a: &str, b: &str) -> usize {
    a.len() + b.len()
}

fn grown(mut a: String, b: &str) -> usize {
    a.push_str(b);
    a.len() + b.len()
}
`,
    calls: [`check("summed", || summed(&[1, 2], &[3]))`, `check("text summed", || text_summed("ab", "cde"))`, `check("grown", || grown(String::from("ab"), "cde"))`],
  },
  // An array's length is its type's: `keys[..6]` of a `[i32; 11]`.
  array_typed: {
    fns: `fn head(keys: &[i32; 11]) -> i32 {
    keys[..6].iter().sum()
}

fn head_of(keys: &[i32; 11], k: usize) -> i32 {
    keys[..k].iter().sum()
}
`,
    calls: [`check("head", || head(&[1; 11]))`, `check("head of 12", || head_of(&[1; 11], 12))`],
  },
  // A match candidate's imaginary edge is no way into an arm: of `1 =>`,
  // the length is one. Not past a clear in the arm.
  match_arms: {
    fns: `fn arm(list: &[u32]) -> u32 {
    match list.len() {
        0 => 0,
        1 => list[0],
        _ => list[1],
    }
}

fn cleared_in_its_arm(list: &mut Vec<u32>) -> u32 {
    match list.len() {
        0 => 0,
        1 => {
            list.clear();
            list[0]
        }
        _ => list[1],
    }
}
`,
    calls: [`check("arm", || arm(&[7]))`, `check("cleared in its arm", || cleared_in_its_arm(&mut vec![7]))`],
  },
  // A counter a `continue` brings back as it was is widened to the
  // constants the body compares it with, not its type's: its `+ 1` never
  // wraps. Not past a write through a `&mut` of it.
  widened: {
    fns: `fn counted() -> u32 {
    let mut count = 0u32;
    let mut i = 0;
    while i < 20 {
        i += 1;
        if count > 10 {
            continue;
        }
        count += 1;
    }
    count
}

fn most(n: &mut u32) {
    *n = u32::MAX;
}

fn written_between() -> u32 {
    let mut count = 0u32;
    let mut i = 0;
    while i < 20 {
        i += 1;
        if count > 10 {
            continue;
        }
        most(&mut count);
        count = count.wrapping_add(0);
        count += 1;
    }
    count
}
`,
    calls: [`check("counted", counted)`, `check("written between", written_between)`],
  },
  // `n == 1` of a held length raises its slice's least: `keys[0]`. Not of
  // the slice made another between.
  held_length: {
    fns: `fn first(keys: &[u32]) -> u32 {
    let n = keys.len();
    if n == 1 { keys[0] } else { 0 }
}

fn first_of_another(keys: &[u32]) -> u32 {
    let mut keys = keys;
    let n = keys.len();
    if n == 1 {
        keys = &[];
        keys[0]
    } else {
        0
    }
}
`,
    calls: [`check("first", || first(&[5]))`, `check("first of another", || first_of_another(&[5]))`],
  },
  // An index a slice's `enumerate()` gives, and past `skip(n)`, is below
  // that slice's length: not another's.
  enumerated: {
    fns: `fn total(terraces: &[u32]) -> u32 {
    let mut total = 0;
    for (i, _) in terraces.iter().enumerate() {
        total += terraces[i];
    }
    total
}

fn skipped(candidates: &[u32]) -> u32 {
    let mut total = 0;
    for (index, _) in candidates.iter().enumerate().skip(1) {
        total += candidates[index];
    }
    total
}

fn of_another(a: &[u32], b: &[u32]) -> u32 {
    let mut total = 0;
    for (i, _) in a.iter().enumerate() {
        total += b[i];
    }
    total
}

fn skipped_of_another(a: &[u32], b: &[u32]) -> u32 {
    let mut total = 0;
    for (index, _) in a.iter().enumerate().skip(1) {
        total += b[index];
    }
    total
}
`,
    calls: [
      `check("total", || total(&[1, 2, 3]))`,
      `check("skipped", || skipped(&[1, 2, 3]))`,
      `check("of another", || of_another(&[1, 2, 3], &[1]))`,
      `check("skipped of another", || skipped_of_another(&[1, 2, 3], &[1, 2]))`,
    ],
  },
  // An index an item was given at is below the length after: a drain to
  // it. Not past a callback that may clear the cell.
  given_back: {
    fns: `fn settled(q: &RefCell<Vec<u32>>, i: usize) -> usize {
    q.borrow_mut()[i] = 0;
    q.borrow_mut().drain(0..i).count()
}

fn cleared_between(q: &RefCell<Vec<u32>>, i: usize, cut: &dyn Fn()) -> usize {
    q.borrow_mut()[i] = 0;
    cut();
    q.borrow_mut().drain(0..i).count()
}
`,
    calls: [
      `check("settled", || settled(&RefCell::new(vec![1, 2, 3]), 2))`,
      `check("cleared between", || { let q = RefCell::new(vec![1, 2, 3]); cleared_between(&q, 2, &|| q.borrow_mut().clear()) })`,
    ],
  },
  // A quotient by a constant is at most what it divides: `n / 2 + n / 4`
  // never wraps.
  quotients: {
    fns: `fn halves(n: u32) -> u32 {
    n / 2 + n / 4
}
`,
    calls: [`check("halves", || halves(u32::MAX))`],
  },
  // A generic constant bounds its loop: `i += 1` below `N` never wraps.
  // Not past a write through a `&mut` of the counter.
  generic_bound: {
    fns: `fn up_to<const N: usize>() -> usize {
    let mut i = 0;
    while i < N {
        i += 1;
    }
    i
}

fn most(n: &mut usize) {
    *n = usize::MAX;
}

fn written_up_to<const N: usize>() -> usize {
    let mut i = 0;
    while i < N {
        most(&mut i);
        i += 1;
        return i;
    }
    i
}
`,
    calls: [`check("up to", up_to::<4>)`, `check("written up to", written_up_to::<4>)`],
  },
  // A counter down from a length: `a[s - 1]` under `s > 0`, its relation
  // kept through the loop's join. Not of the slice made shorter in it.
  counted_down: {
    fns: `fn down(a: &[u32]) -> u32 {
    let mut total = 0;
    let mut s = a.len();
    while s > 0 {
        total += a[s - 1];
        s -= 1;
    }
    total
}

fn down_of_shorter(a: &[u32]) -> u32 {
    let mut a = a;
    let mut total = 0;
    let mut s = a.len();
    while s > 0 {
        a = &a[..1];
        total += a[s - 1];
        s -= 1;
    }
    total
}
`,
    calls: [`check("down", || down(&[1, 2, 3]))`, `check("down of shorter", || down_of_shorter(&[1, 2, 3]))`],
  },
  // What a value is below bounds its interval: `v[i >> 1]` under
  // `i < v.len()`. Not of the slice made another between.
  shifted: {
    fns: `fn half(v: &[u32], i: usize) -> u32 {
    if i < v.len() { v[i >> 1] } else { 0 }
}

fn half_of_another(v: &[u32], i: usize) -> u32 {
    let mut v = v;
    if i < v.len() {
        v = &[];
        v[i >> 1]
    } else {
        0
    }
}
`,
    calls: [`check("half", || half(&[1, 2, 3, 4], 3))`, `check("half of another", || half_of_another(&[1, 2, 3, 4], 3))`],
  },
  // `vec![x; v.len()]` is as long as `v`. Not past a pop of it.
  as_long: {
    fns: `fn left(terraces: &[u32], k: usize) -> u32 {
    let left = vec![0u32; terraces.len()];
    if k < terraces.len() { left[k] } else { 9 }
}

fn popped(terraces: &[u32], k: usize) -> u32 {
    let mut left = vec![0u32; terraces.len()];
    if k < terraces.len() {
        left.pop();
        left[k]
    } else {
        9
    }
}
`,
    calls: [`check("left", || left(&[1, 2, 3], 2))`, `check("popped", || popped(&[1, 2, 3], 2))`],
  },
  // A macro's repeated index, one in bounds and one not: each checked as
  // its own (MIR's bounds check is joined to its own index).
  repeated: {
    fns: `macro_rules! pick {
    ($v:ident, $($i:expr),*) => {
        [$($v[$i]),*]
    };
}

fn picked(v: &[u32; 4], k: usize) -> [u32; 2] {
    pick!(v, 0, k)
}
`,
    calls: [`check("picked", || picked(&[1, 2, 3, 4], 1))`, `check("picked past", || picked(&[1, 2, 3, 4], 9))`],
  },
  // The unsound direction's shapes, each the fact its rule forgets kept:
  // a recursive function's parameter given larger by a caller outside, a
  // parameter its own calls keep raising, a `&mut` of a `Vec` moved to
  // another local and the `Vec` shortened through it, a capture its
  // closure's body writes, a cell's `Vec` replaced through a raw pointer,
  // a `Vec` pushed then cleared through a pointer or a function, a map's
  // key written past its test, a generic constant's loop past its bound,
  // and `..=n` of `n <= len`; a recursive function's parameter its own
  // calls give one value and its caller outside another, `down(5, d - 1)`
  // of `down(0, 0)`, and a cell's `Vec` emptied under `!is_empty()`
  // through a pointer it's given, with no call, whose root isn't the cell
  // (#1943), and indexed so, a call's index MIR decides (#1944). Each runs
  // as native does only where its rule forgets; and a `Vec` a closure
  // borrows `&mut` and shortens, its length read before (#1946).
  unsound_directions: {
    fns: `fn walk(k: u32, d: u32) -> u32 {
    if d == 0 {
        return k + 1;
    }
    walk(k, d - 1)
}

fn up(n: u8, d: u32) -> u8 {
    if d == 0 { n } else { up(n + 1, d - 1) }
}

fn moved(mut v: Vec<u32>, i: usize) -> u32 {
    if i < v.len() {
        let r = &mut v;
        let s = r;
        s.truncate(1);
        return v[i];
    }
    0
}

fn moved_wrap(mut v: Vec<u32>) -> u32 {
    if v.len() > 1 {
        let r = &mut v;
        let s = r;
        s.truncate(1);
        return (v.len() - 2) as u32;
    }
    9
}

fn counted_down() -> u32 {
    let mut at = 2u32;
    let mut f = move || {
        at -= 1;
        at
    };
    f();
    f();
    f()
}

fn replaced(cell: &RefCell<Vec<u32>>, i: usize) -> u32 {
    let p: *mut Vec<u32> = {
        let mut guard = cell.borrow_mut();
        &mut *guard
    };
    if i < cell.borrow().len() {
        unsafe { *p = Vec::new() };
        return cell.borrow()[i];
    }
    0
}

fn replaced_wrap(cell: &RefCell<Vec<u32>>) -> u32 {
    let p: *mut Vec<u32> = {
        let mut guard = cell.borrow_mut();
        &mut *guard
    };
    if cell.borrow().len() > 1 {
        unsafe { *p = vec![9] };
        return (cell.borrow().len() - 2) as u32;
    }
    9
}
fn pushed_then_cleared(mut v: Vec<u32>) -> u32 {
    v.push(1);
    let p: *mut Vec<u32> = &raw mut v;
    unsafe { (&mut *p).clear() };
    (v.len() - 1) as u32
}

fn clear_it(v: &mut Vec<u32>) {
    v.clear();
}

fn pushed_then_lent(mut v: Vec<u32>) -> u32 {
    v.push(1);
    clear_it(&mut v);
    (v.len() - 1) as u32
}

fn key_written(m: &HashMap<u32, u32>, mut k: u32) -> u32 {
    if m.contains_key(&k) {
        k += 1;
        return m[&k];
    }
    0
}

fn down(n: u32, d: u32) -> u32 {
    if d == 0 {
        return n - 1;
    }
    down(5, d - 1)
}
fn emptied_through_a_pointer(cell: &RefCell<Vec<u32>>, p: *mut Vec<u32>, empty: Vec<u32>) -> u32 {
    if !cell.borrow().is_empty() {
        unsafe { *p = empty };
        return (cell.borrow().len() - 1) as u32;
    }
    9
}
fn indexed_through_a_pointer(cell: &RefCell<Vec<u32>>, p: *mut Vec<u32>, empty: Vec<u32>, i: usize) -> u32 {
    if i < cell.borrow().len() {
        unsafe { *p = empty };
        return cell.borrow()[i];
    }
    9
}
fn taken_by_a_closure(mut args: Vec<u32>, i: usize) -> u32 {
    let n = args.len();
    let mut take = || args.pop();
    take();
    if i < n { args[i] } else { 9 }
}
fn pointer_to(cell: &RefCell<Vec<u32>>) -> *mut Vec<u32> {
    let mut guard = cell.borrow_mut();
    &mut *guard
}
fn past_its_bound<const N: usize>() -> u32 {
    let mut i = 0u32;
    while (i as usize) < N {
        i += 1;
    }
    i - 1
}

fn through_its_end(v: &[u32], n: usize) -> usize {
    if n <= v.len() { v[..=n].len() } else { 0 }
}
`,
    calls: [
      `check("walk", || walk(3, 2))`,
      `check("walk from the top", || walk(u32::MAX, 2))`,
      `check("up", || up(250, 10))`,
      `check("moved", || moved(vec![1, 2, 3], 2))`,
      `check("moved wrap", || moved_wrap(vec![1, 2, 3]))`,
      `check("counted down", counted_down)`,
      `check("replaced", || replaced(&RefCell::new(vec![1, 2, 3]), 2))`,
      `check("replaced wrap", || replaced_wrap(&RefCell::new(vec![1, 2, 3])))`,
      `check("pushed then cleared", || pushed_then_cleared(vec![]))`,
      `check("pushed then lent", || pushed_then_lent(vec![]))`,
      `check("key written", || key_written(&HashMap::from([(1, 10)]), 1))`,
      `check("down", || down(0, 0))`,
      `check("down twice", || down(0, 2))`,
      `check("emptied through a pointer", || {
        let cell = RefCell::new(vec![1, 2]);
        emptied_through_a_pointer(&cell, pointer_to(&cell), Vec::new())
    })`,
      `check("indexed through a pointer", || {
        let cell = RefCell::new(vec![1, 2]);
        indexed_through_a_pointer(&cell, pointer_to(&cell), Vec::new(), 1)
    })`,
      `check("taken by a closure", || taken_by_a_closure(vec![1, 2], 1))`,
      `check("past its bound", past_its_bound::<0>)`,
      `check("through its end", || through_its_end(&[1, 2, 3], 3) as u32)`,
    ],
  },
  // The corpus's own shapes of five rules: a counter a `break` at a
  // constant leaves, `k += 1` past `k == 60`; a slice's first index raising
  // its least, `bytes[bytes.len() - 1]` past `bytes[0]`; a search's `v[i]`
  // under `i != v.len()`, its relation kept through the loop's join;
  // `i += 1` under `i < n` of a parameter, and past `signs[i]`, below its
  // length; a macro's `$v[$i]` repeated of
  // `k` and `k + 1` under `k < v.len()`. Each beside the write between that
  // breaks it.
  corpus_shapes: {
    fns: `fn most(n: &mut u32) {
    *n = u32::MAX;
}

fn to_sixty() -> u32 {
    let mut k = 0u32;
    loop {
        if k == 60 {
            break;
        }
        k += 1;
    }
    k
}

fn to_sixty_written() -> u32 {
    let mut k = 0u32;
    loop {
        if k == 60 {
            break;
        }
        most(&mut k);
        k += 1;
        return k;
    }
    k
}

fn ends(bytes: &[u8]) -> (u8, u8) {
    (bytes[0], bytes[bytes.len() - 1])
}

fn ends_of_another(bytes: &[u8]) -> (u8, u8) {
    let mut bytes = bytes;
    let first = bytes[0];
    bytes = &[];
    // Through a u32, so its index is the same as JS's usize's.
    (first, bytes[(bytes.len() as u32).wrapping_sub(1) as usize])
}

fn found(words: &[&str]) -> usize {
    let mut i = 0;
    loop {
        if i == words.len() || words[i] == "stop" {
            break;
        }
        i += 1;
    }
    i
}

fn found_of_shorter(words: &[&str]) -> usize {
    let mut words = words;
    let mut i = 0;
    loop {
        if i == words.len() || words[i] == "stop" {
            break;
        }
        words = &words[..1];
        i += 1;
    }
    i
}

fn counted_to(n: usize) -> usize {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}

fn most_of(n: &mut usize) {
    *n = usize::MAX;
}

fn counted_to_written(n: usize) -> usize {
    let mut i = 0;
    while i < n {
        most_of(&mut i);
        i += 1;
        return i;
    }
    i
}

fn filled(items: &[i8]) -> Vec<i8> {
    let mut signs = vec![0i8; items.len()];
    let mut i = 0;
    for &n in items {
        signs[i] = -n;
        i += 1;
    }
    signs
}

fn filled_written(items: &[i8]) -> usize {
    let mut signs = vec![0i8; items.len()];
    let mut i = 0;
    for &n in items {
        signs[i] = -n;
        most_of(&mut i);
        i += 1;
        return i;
    }
    i
}

macro_rules! pick {
    ($v:expr; $($i:expr),*) => {
        [$($v[$i]),*]
    };
}

fn pair(v: &[i32], k: usize) -> [i32; 2] {
    if k < v.len() { pick!(v; k, k + 1) } else { [0, 0] }
}
`,
    calls: [
      `check("to sixty", to_sixty)`,
      `check("to sixty written", to_sixty_written)`,
      `check("ends", || ends(&[1, 2, 3]))`,
      `check("ends of another", || ends_of_another(&[1, 2, 3]))`,
      `check("found", || found(&["a", "stop"]))`,
      `check("found of shorter", || found_of_shorter(&["a", "b", "c"]))`,
      `check("counted to", || counted_to(5))`,
      `check("counted to written", || counted_to_written(5))`,
      `check("filled", || filled(&[4, -2, 7]))`,
      `check("filled written", || filled_written(&[4, -2, 7]))`,
      `check("pair", || pair(&[1, 2, 3], 1))`,
      `check("pair past", || pair(&[1, 2, 3], 2))`,
    ],
  },
  // A length held before a `push` is below the length past it, and one
  // tested equal to a length is at most it: `v[n]`, `v[n - 1]` (ADR 1168,
  // #1948). Not past a `clear()`, a `pop()` or the vector made another,
  // nor of a length indexed at, or another slice's. A held length that may
  // be 0, whose `n - 1` wraps, is corpus case `length_shifted_past_a_push`'s,
  // which prints no index, `usize::MAX` JS's 32 bits.
  pushed_length: {
    fns: `fn pushed(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v[n]
}

fn pushed_then_cleared(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v.clear();
    v[n]
}

fn pushed_then_popped(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v.pop();
    v[n]
}

fn pushed_then_replaced(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    *v = Vec::new();
    v[n]
}

fn equal(v: &[u32], n: usize) -> u32 {
    if n >= 1 && n == v.len() { v[n - 1] } else { 0 }
}

fn equal_at(v: &[u32], n: usize) -> u32 {
    if n == v.len() { v[n] } else { 0 }
}

fn below_another(a: &[u32], b: &mut Vec<u32>, i: usize) -> u32 {
    if i < a.len() {
        b.push(7);
        b[i]
    } else {
        0
    }
}
`,
    calls: [
      `check("pushed", || pushed(&mut vec![1, 2]))`,
      `check("pushed then cleared", || pushed_then_cleared(&mut vec![1]))`,
      `check("pushed then popped", || pushed_then_popped(&mut vec![1]))`,
      `check("pushed then replaced", || pushed_then_replaced(&mut vec![1]))`,
      `check("equal", || equal(&[1, 2], 2))`,
      `check("equal at", || equal_at(&[1, 2], 2))`,
      `check("below another", || below_another(&[1, 2, 3], &mut Vec::new(), 2))`,
    ],
  },
  // A `RefCell`'s length across a `for` loop's steps, its range's `next()`
  // std's, which writes no cell: `batch.borrow()[i]` of `i` in `(0..
  // batch.borrow().len()).step_by(2)` (ADR 1168, #1948). Not past a
  // `borrow_mut().clear()` in the loop of a held length, nor `0..=len`.
  cell_steps: {
    fns: `fn stepped(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i];
    }
    total
}

fn held_cleared(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    let end = batch.borrow().len();
    for i in (0..end).step_by(2) {
        total += batch.borrow()[i];
        batch.borrow_mut().clear();
    }
    total
}

fn past_the_end(batch: &RefCell<Vec<u32>>) -> u32 {
    let mut total = 0;
    for i in (0..=batch.borrow().len()).step_by(2) {
        total += batch.borrow()[i];
    }
    total
}
`,
    calls: [
      `check("stepped", || stepped(&RefCell::new(vec![1, 2, 3, 4, 5])))`,
      `check("held cleared", || held_cleared(&RefCell::new(vec![1, 2, 3])))`,
      `check("past the end", || past_the_end(&RefCell::new(vec![1, 2])))`,
    ],
  },
  // A slice's `get(i)`'s `Some` puts `i` below its length: `i += 1` never
  // wraps.
  got: {
    fns: `fn walked(items: &[u32]) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while let Some(&item) = items.get(i) {
        total += item;
        i += 1;
    }
    total
}
`,
    calls: [`check("walked", || walked(&[1, 2, 3]))`],
  },
  // What std's code writes (#2112 B 3b): a `&mut Vec` std's `push`
  // stores into another `Vec`, cleared through it, its length forgotten;
  // and the crate's `Ord`, which std's `sort` and `max` run by their
  // bound `T: Ord`, clearing a cell a thread-local shares with a
  // parameter: each runs as native does only where what std's code does is
  // asked of it.
  std_code_writes: {
    fns: `use std::cmp::Ordering;
use std::rc::Rc;

thread_local! {
    static HELD: RefCell<Option<Rc<RefCell<Vec<u32>>>>> = const { RefCell::new(None) };
}

fn clear_held() {
    HELD.with(|h| {
        if let Some(c) = &*h.borrow() {
            c.borrow_mut().clear();
        }
    });
}

#[derive(PartialEq, Eq)]
struct Key(u32);

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        clear_held();
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn stored_cleared(k: usize) -> u32 {
    let mut x = vec![1u32, 2, 3];
    if k >= x.len() {
        return 0;
    }
    {
        let mut v: Vec<&mut Vec<u32>> = Vec::new();
        v.push(&mut x);
        v[0].clear();
    }
    x[k]
}

fn sorted_clears(cell: &RefCell<Vec<u32>>, k: usize) -> u32 {
    if k >= cell.borrow().len() {
        return 0;
    }
    let mut keys = vec![Key(2), Key(1)];
    keys.sort();
    cell.borrow()[k]
}

fn max_clears(cell: &RefCell<Vec<u32>>, k: usize) -> u32 {
    if k >= cell.borrow().len() {
        return 0;
    }
    let m = std::cmp::max(Key(1), Key(2));
    cell.borrow()[k] + m.0
}

fn shared() -> Rc<RefCell<Vec<u32>>> {
    let cell = Rc::new(RefCell::new(vec![1u32, 2, 3]));
    HELD.with(|h| *h.borrow_mut() = Some(Rc::clone(&cell)));
    cell
}
`,
    calls: [
      `check("stored_cleared 0", || stored_cleared(0))`,
      `check("stored_cleared 1", || stored_cleared(1))`,
      `check("sorted_clears 1", || sorted_clears(&shared(), 1))`,
      `check("max_clears 1", || max_clears(&shared(), 1))`,
    ],
  },
};

/** The shape family's programs, `shape_<name>`. */
export function shapePrograms(): Record<string, string> {
  return Object.fromEntries(
    Object.entries(shapes).map(([name, s]) => [`shape_${name}`, `${shapeDecls}\n${s.fns}\nfn main() {\n${s.calls.map((c) => `    ${c};`).join("\n")}\n}\n`]),
  );
}
