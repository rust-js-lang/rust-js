// The bounds matrix (ADR 1137, the audit before step 5's switch): a
// collection's length tested, then something run that may change it,
// then an index of it, each holder of the collection meeting each test,
// each thing between and each form of index, that rust-js must run as
// native Rust does. What the MIR intervals prove of a length is sound
// only while nothing between can change it: another path to the same
// collection (another `Rc`, a shared `&RefCell`, a field), a callback, a
// closure, a reassignment, a `&mut` lent, a drop. Each program is one
// holder and one test, a function for each thing between and each form.

import { shapePrograms } from "./bounds-matrix-shapes";

/** Where a collection lives, as the probe sees it, and every way to reach
 * it again. `len` and `at` read it; the rest, each where the holder
 * admits it, shorten it to one item. */
type Holder = {
  /** The probe's parameters but `k` and `cut`. */
  params: string;
  /** Statements before the test, inside the probe. */
  setup?: string;
  /** In `main`: statements making a fresh collection of 1, 2, 3, 4, and
   * the probe's arguments but `k` and `cut`. */
  make: string;
  args: string;
  /** In `main`: a closure body that shortens it through another path. */
  cut?: string;
  len: string;
  at: (k: string) => string;
  /** Its items are bytes, an item made a `u32` to be the probe's. */
  bytes?: boolean;
  /** A call's index of it, `swap`, and a drain to one past an index,
   * where it's mutable. */
  swap?: (k: string) => string;
  drain?: (k: string) => string;
  /** Inside the probe: a closure body that shortens it through what the
   * probe holds shared. */
  inner?: string;
  direct?: string;
  reassign?: string;
  lend?: string;
  whole?: string;
  /** A call of a function of the crate's that shortens it through the
   * shared path, `shorten_cell(v)`: a call the walk can name, not a
   * callback. */
  helper?: string;
  /** An item's write through the same reference, `v[0] ^= 1`, which
   * keeps the length; and the whole swapped through it, `mem::swap(&mut
   * *v, ..)`, which doesn't. */
  item?: string;
  swapped?: string;
  /** A write through a raw pointer to it, `p` of its setup. */
  pointer?: string;
  /** A guard's `&mut` kept in a struct's field and the `Vec` shortened
   * through it, and through a raw pointer made of it, by a call and by
   * `ptr::write`: what a held guard's `deref_mut` lends, used other than
   * given whole to a call (ADR 1137, the guard's own rule retired). */
  stored?: string;
  pointer_write?: string;
  /** The guard's `&mut` bound to a local and the `Vec` shortened through it,
   * `let all = &mut *g; all.truncate(1)`: its call given a reborrow of the
   * local, no root of the cell's, so only the `deref_mut`'s own rule
   * forgets the length (#1928). */
  bound?: string;
  /** The tests, or a test's form, `<test>/<form>`, whose programs are
   * undefined behaviour under Miri, so no answer of theirs is wrong:
   * each left out, with what Miri says beside it, so none is added again. */
  undefined?: string[];
};

const decls = `use std::any::Any;
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check(what: &str, f: impl FnOnce() -> u32) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn shorten(v: &mut Vec<u32>) {
    v.truncate(1);
}

fn shorten_cell(v: &RefCell<Vec<u32>>) {
    v.borrow_mut().truncate(1);
}

fn shorten_local() {
    ITEMS.with_borrow_mut(|v| v.truncate(1));
}

fn shorten_text(s: &mut String) {
    s.truncate(1);
}

fn shorten_slice<'a>(v: &mut &'a [u32]) {
    let s: &'a [u32] = v;
    *v = &s[..1];
}

fn replace_rc(v: &mut Rc<RefCell<Vec<u32>>>) {
    *v = Rc::new(RefCell::new(vec![9]));
}

struct Holder {
    items: Vec<u32>,
}

fn reset(h: &mut Holder) {
    h.items.truncate(1);
}

/// A struct whose field is a slice, whose index is a bounds assert.
struct Pair {
    items: &'static [u32],
}

fn repoint(h: &mut &Pair) {
    *h = Box::leak(Box::new(Pair { items: &[9] }));
}

/// A struct that keeps a &mut of a Vec.
struct Lent<'a> {
    v: &'a mut Vec<u32>,
}

struct Shared {
    items: RefCell<Vec<u32>>,
}

fn shorten_rows(t: &mut Vec<Vec<u32>>) {
    for row in t.iter_mut() {
        row.truncate(1);
    }
}

/// A value whose drop runs what it's given.
struct OnDrop<'a>(&'a dyn Fn());

impl Drop for OnDrop<'_> {
    fn drop(&mut self) {
        (self.0)();
    }
}

thread_local! {
    static ITEMS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}
`;

const four = "vec![1u32, 2, 3, 4]";

const holders: Record<string, Holder> = {
  vec_owned: {
    params: "mut v: Vec<u32>",
    make: "",
    args: four,
    len: "v.len()",
    at: (k) => `v[${k}]`,
    swap: (k) => `{ v.swap(${k}, 0); v[0] }`,
    drain: (k) => `v.drain(..${k} + 1).count() as u32`,
    direct: "v.truncate(1);",
    reassign: "v = vec![9];",
    lend: "shorten(&mut v);",
  },
  slice: {
    params: "mut v: &[u32]",
    make: `let items = ${four};`,
    args: "&items",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    reassign: "v = &[9];",
    lend: "shorten_slice(&mut v);",
  },
  text: {
    params: "mut s: String",
    make: "",
    args: `String::from("abcd")`,
    len: "s.len()",
    at: (k) => `s.as_bytes()[${k}]`,
    bytes: true,
    direct: "s.truncate(1);",
    reassign: `s = String::from("x");`,
    lend: "shorten_text(&mut s);",
  },
  text_ref: {
    params: "mut s: &str",
    make: "",
    args: `"abcd"`,
    len: "s.len()",
    at: (k) => `s.as_bytes()[${k}]`,
    bytes: true,
    reassign: `s = "x";`,
  },
  vec_mut: {
    params: "v: &mut Vec<u32>",
    item: "v[0] ^= 1;",
    swapped: "std::mem::swap(&mut *v, &mut vec![9]);",
    make: `let mut items = ${four};`,
    args: "&mut items",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    swap: (k) => `{ v.swap(${k}, 0); v[0] }`,
    drain: (k) => `v.drain(..${k} + 1).count() as u32`,
    direct: "v.truncate(1);",
    reassign: "*v = vec![9];",
    lend: "shorten(v);",
  },
  slice_items: {
    params: "v: &mut [u32]",
    make: `let mut items = ${four};`,
    args: "&mut items",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    swap: (k) => `{ v.swap(${k}, 0); v[0] }`,
    item: "v[0] ^= 1;",
  },
  slice_mut: {
    params: "v: &mut &[u32]",
    swapped: "std::mem::swap(v, &mut &[9][..]);",
    make: `let items = ${four}; let mut s = &items[..];`,
    args: "&mut s",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    reassign: "*v = &[9];",
    lend: "shorten_slice(v);",
  },
  slice_field: {
    params: "mut h: &Pair",
    make: `let held = Pair { items: Vec::leak(${four}) };`,
    args: "&held",
    len: "h.items.len()",
    at: (k) => `h.items[${k}]`,
    reassign: "h = Box::leak(Box::new(Pair { items: &[9] }));",
    lend: "repoint(&mut h);",
  },
  slice_field_mut: {
    params: "h: &mut Pair",
    make: `let mut held = Pair { items: Vec::leak(${four}) };`,
    args: "&mut held",
    len: "h.items.len()",
    at: (k) => `h.items[${k}]`,
    reassign: "*h = Pair { items: &[9] };",
    whole: "h.items = &[9];",
  },
  vec_mut_mut: {
    params: "v: &mut &mut Vec<u32>",
    item: "v[0] ^= 1;",
    swapped: "std::mem::swap(&mut **v, &mut vec![9]);",
    make: `let mut items = ${four}; let mut r = &mut items;`,
    args: "&mut r",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    swap: (k) => `{ v.swap(${k}, 0); v[0] }`,
    drain: (k) => `v.drain(..${k} + 1).count() as u32`,
    direct: "v.truncate(1);",
    reassign: "**v = vec![9];",
    lend: "shorten(v);",
  },
  pointer: {
    params: "v: &mut Vec<u32>",
    item: "unsafe { (&mut *p)[0] ^= 1 };",
    setup: "let p: *mut Vec<u32> = &raw mut *v;",
    make: `let mut items = ${four};`,
    args: "&mut items",
    len: "v.len()",
    at: (k) => `v[${k}]`,
    swap: (k) => `{ v.swap(${k}, 0); v[0] }`,
    drain: (k) => `v.drain(..${k} + 1).count() as u32`,
    reassign: "unsafe { *p = vec![9] };",
    pointer: "unsafe { std::ptr::write(p, vec![9]) };",
    // Undefined behaviour, Miri's Stacked Borrows (nightly, 2026-11-15):
    // - `in_closure`: the write through `p` invalidates the closure's
    //   capture of `v`, which `v[k]` then reads: "trying to retag from
    //   <34780> for SharedReadOnly permission ..., but that tag does not
    //   exist in the borrow stack"; "<34780> was created by a
    //   SharedReadOnly retag" at the closure, "later invalidated ... by a
    //   Unique retag" at `*p = vec![9]`.
    // - a loop's `swap` or `drain`: its `&mut` reborrow of `v` invalidates
    //   `p`, which the next step writes through, `loop_for/swap`'s "trying
    //   to retag from <91523> for Unique permission ..., but that tag does
    //   not exist"; "<91523> was created by a SharedReadWrite retag" at
    //   `&raw mut *v`, "later invalidated ... by a Unique retag" at
    //   `v.swap(j, 0)`; `loop_step/swap`'s the same. Each loop's is left
    //   out, `loop_rev`'s and `loop_while`'s too, which Miri passes only
    //   as their first step panics before a second.
    undefined: ["in_closure", ...["loop_for", "loop_rev", "loop_step", "loop_while"].flatMap((loop) => [`${loop}/swap`, `${loop}/drain`])],
  },
  local_field: {
    params: "items: Vec<u32>",
    item: "x.items[0] ^= 1;",
    swapped: "std::mem::swap(&mut x.items, &mut vec![9]);",
    setup: "let mut x = Holder { items };",
    make: "",
    args: four,
    len: "x.items.len()",
    at: (k) => `x.items[${k}]`,
    swap: (k) => `{ x.items.swap(${k}, 0); x.items[0] }`,
    drain: (k) => `x.items.drain(..${k} + 1).count() as u32`,
    direct: "x.items.truncate(1);",
    reassign: "x = Holder { items: vec![9] };",
    lend: "shorten(&mut x.items);",
    whole: "x.items = vec![9];",
  },
  refcell: {
    params: "v: &RefCell<Vec<u32>>",
    make: `let cell = RefCell::new(${four});`,
    args: "&cell",
    cut: "cell.borrow_mut().truncate(1)",
    len: "v.borrow().len()",
    at: (k) => `v.borrow()[${k}]`,
    swap: (k) => `{ v.borrow_mut().swap(${k}, 0); v.borrow()[0] }`,
    drain: (k) => `v.borrow_mut().drain(..${k} + 1).count() as u32`,
    inner: "v.borrow_mut().truncate(1)",
    direct: "v.borrow_mut().truncate(1);",
    helper: "shorten_cell(v);",
    lend: "shorten(&mut v.borrow_mut());",
  },
  rc: {
    params: "mut v: Rc<RefCell<Vec<u32>>>",
    make: `let cell = Rc::new(RefCell::new(${four})); let other = cell.clone();`,
    args: "cell",
    cut: "other.borrow_mut().truncate(1)",
    len: "v.borrow().len()",
    at: (k) => `v.borrow()[${k}]`,
    swap: (k) => `{ v.borrow_mut().swap(${k}, 0); v.borrow()[0] }`,
    drain: (k) => `v.borrow_mut().drain(..${k} + 1).count() as u32`,
    inner: "v.borrow_mut().truncate(1)",
    direct: "v.borrow_mut().truncate(1);",
    helper: "shorten_cell(&v);",
    reassign: "v = Rc::new(RefCell::new(vec![9]));",
    lend: "replace_rc(&mut v);",
  },
  rc_ref: {
    params: "v: &Rc<RefCell<Vec<u32>>>",
    make: `let cell = Rc::new(RefCell::new(${four}));`,
    args: "&cell",
    cut: "cell.borrow_mut().truncate(1)",
    len: "v.borrow().len()",
    at: (k) => `v.borrow()[${k}]`,
    inner: "v.borrow_mut().truncate(1)",
    direct: "v.borrow_mut().truncate(1);",
    helper: "shorten_cell(v);",
  },
  boxed: {
    params: "v: Box<RefCell<Vec<u32>>>",
    make: "",
    args: `Box::new(RefCell::new(${four}))`,
    len: "v.borrow().len()",
    at: (k) => `v.borrow()[${k}]`,
    inner: "v.borrow_mut().truncate(1)",
    direct: "v.borrow_mut().truncate(1);",
    helper: "shorten_cell(&v);",
  },
  guard: {
    params: "v: &RefCell<Vec<u32>>",
    setup: "let mut g = v.borrow_mut();",
    make: `let cell = RefCell::new(${four});`,
    args: "&cell",
    cut: "cell.borrow_mut().truncate(1)",
    len: "g.len()",
    at: (k) => `g[${k}]`,
    swap: (k) => `{ g.swap(${k}, 0); g[0] }`,
    drain: (k) => `g.drain(..${k} + 1).count() as u32`,
    direct: "g.truncate(1);",
    reassign: "*g = vec![9];",
    lend: "shorten(&mut g);",
    stored: "{ let lent = Lent { v: &mut g }; lent.v.truncate(1); }",
    bound: "{ let all = &mut *g; all.truncate(1); }",
    pointer: "{ let r: &mut Vec<u32> = &mut g; let p: *mut Vec<u32> = r; unsafe { (&mut *p).truncate(1) }; }",
    pointer_write: "{ let r: &mut Vec<u32> = &mut g; let p: *mut Vec<u32> = r; unsafe { std::ptr::write(p, vec![9]) }; }",
  },
  field: {
    params: "mut h: &Holder",
    make: `let held = Holder { items: ${four} };`,
    args: "&held",
    len: "h.items.len()",
    at: (k) => `h.items[${k}]`,
    reassign: "h = Box::leak(Box::new(Holder { items: vec![9] }));",
  },
  field_mut: {
    params: "h: &mut Holder",
    item: "h.items[0] ^= 1;",
    swapped: "std::mem::swap(&mut h.items, &mut vec![9]);",
    make: `let mut held = Holder { items: ${four} };`,
    args: "&mut held",
    len: "h.items.len()",
    at: (k) => `h.items[${k}]`,
    swap: (k) => `{ h.items.swap(${k}, 0); h.items[0] }`,
    drain: (k) => `h.items.drain(..${k} + 1).count() as u32`,
    direct: "h.items.truncate(1);",
    reassign: "*h = Holder { items: vec![9] };",
    lend: "shorten(&mut h.items);",
    whole: "reset(h);",
  },
  field_cell: {
    params: "h: &Shared",
    make: `let shared = Shared { items: RefCell::new(${four}) };`,
    args: "&shared",
    cut: "shared.items.borrow_mut().truncate(1)",
    len: "h.items.borrow().len()",
    at: (k) => `h.items.borrow()[${k}]`,
    inner: "h.items.borrow_mut().truncate(1)",
    direct: "h.items.borrow_mut().truncate(1);",
    helper: "shorten_cell(&h.items);",
  },
  table: {
    params: "v: &Vec<u32>",
    setup: "let mut t = vec![v.clone(); 3];",
    make: `let items = ${four};`,
    args: "&items",
    len: "t[0].len()",
    at: (k) => `t[1][${k}]`,
    swap: (k) => `{ t[1].swap(${k}, 0); t[1][0] }`,
    drain: (k) => `t[1].drain(..${k} + 1).count() as u32`,
    direct: "t[1].truncate(1);",
    reassign: "t[1] = vec![9];",
    lend: "shorten(&mut t[1]);",
    whole: "shorten_rows(&mut t);",
  },
  table_slice: {
    params: "v: &Vec<u32>",
    item: "s[1][0] ^= 1;",
    swapped: "std::mem::swap(&mut s[1], &mut vec![9]);",
    setup: "let mut t = vec![v.clone(); 3];\n    let s: &mut [Vec<u32>] = &mut t;",
    make: `let items = ${four};`,
    args: "&items",
    len: "s[0].len()",
    at: (k) => `s[1][${k}]`,
    swap: (k) => `{ s[1].swap(${k}, 0); s[1][0] }`,
    drain: (k) => `s[1].drain(..${k} + 1).count() as u32`,
    direct: "s[1].truncate(1);",
    reassign: "s[1] = vec![9];",
    lend: "shorten(&mut s[1]);",
  },
  thread_local: {
    params: "",
    make: `ITEMS.set(${four});`,
    args: "",
    cut: "ITEMS.with_borrow_mut(|v| v.truncate(1))",
    len: "ITEMS.with_borrow(|v| v.len())",
    at: (k) => `ITEMS.with_borrow(|v| v[${k}])`,
    inner: "ITEMS.with_borrow_mut(|v| v.truncate(1))",
    direct: "ITEMS.with_borrow_mut(|v| v.truncate(1));",
    helper: "shorten_local();",
  },
};

/** What runs between the test and the index, each of a holder's ways to
 * shorten it, and nothing, the control. */
const betweens: Record<string, (h: Holder) => string | undefined> = {
  nothing: () => "",
  callback: (h) => (h.cut ? "cut();" : undefined),
  dropped: (h) => (h.cut ? "drop(OnDrop(cut));" : undefined),
  closure: (h) => (h.inner ? `let shorten_it = || ${h.inner}; shorten_it();` : undefined),
  direct: (h) => h.direct,
  helper: (h) => h.helper,
  item: (h) => h.item,
  swapped: (h) => h.swapped,
  reassign: (h) => h.reassign,
  lend: (h) => h.lend,
  whole: (h) => h.whole,
  pointer: (h) => h.pointer,
  stored: (h) => h.stored,
  bound: (h) => h.bound,
  pointer_write: (h) => h.pointer_write,
};

/** The index's form: an item, a range's end, a call's (`swap`). */
const forms: Record<string, (h: Holder, k: string) => string | undefined> = {
  item: (h, k) => (h.bytes ? `u32::from(${h.at(k)})` : h.at(k)),
  range: (h, k) => {
    const at = h.at(`..${k} + 1`);
    return at.includes("with_borrow") ? at.replace(/\)$/, ".len() as u32)") : `${at}.len() as u32`;
  },
  swap: (h, k) => h.swap?.(k),
  drain: (h, k) => h.drain?.(k),
};

/** The test, around what's between and the index, `idx(k)`. */
const tests: Record<string, (h: Holder, between: string, idx: (k: string) => string, name: string) => string> = {
  below: (h, b, idx) => `if k < ${h.len} {\n        ${b}\n        ${idx("k")}\n    } else {\n        0\n    }`,
  held: (h, b, idx) => `let ok = k < ${h.len};\n    if ok {\n        ${b}\n        ${idx("k")}\n    } else {\n        0\n    }`,
  next: (h, b, idx) => `if k + 1 < ${h.len} {\n        ${b}\n        ${idx("k + 1")}\n    } else {\n        0\n    }`,
  last: (h, b, idx) => `let n = ${h.len};\n    if n > k {\n        ${b}\n        ${idx("n - 1")}\n    } else {\n        0\n    }`,
  // A length less what it's known above, which wraps once something
  // between shortens it to one.
  wrap: (h, b) => `if ${h.len} > 1 {\n        ${b}\n        (${h.len} - 2) as u32\n    } else {\n        0\n    }`,
  // `k != len` beside `k <= len`, which together are `k < len`.
  unequal: (h, b, idx) => `if k <= ${h.len} && k != ${h.len} {\n        ${b}\n        ${idx("k")}\n    } else {\n        0\n    }`,
  // The least of it and the last index.
  least: (h, b, idx) => `if ${h.len} > 0 {\n        let j = k.min(${h.len} - 1);\n        ${b}\n        ${idx("j")}\n    } else {\n        0\n    }`,
  // A remainder of the length, below it.
  remainder: (h, b, idx) => `let n = ${h.len};\n    if n > 0 {\n        ${b}\n        ${idx("k % n")}\n    } else {\n        0\n    }`,
  loop_while: (h, b, idx) =>
    `let mut total = 0;\n    let mut j = k;\n    while j < ${h.len} {\n        ${b}\n        total += ${idx("j")};\n        j += 1;\n    }\n    total`,
  loop_for: (h, b, idx) => `let mut total = 0;\n    let end = ${h.len};\n    for j in k..end {\n        ${b}\n        total += ${idx("j")};\n    }\n    total`,
  loop_rev: (h, b, idx) => `let mut total = 0;\n    let end = ${h.len};\n    for j in (k..end).rev() {\n        ${b}\n        total += ${idx("j")};\n    }\n    total`,
  loop_step: (h, b, idx) => `let mut total = 0;\n    let end = ${h.len};\n    for j in (k..end).step_by(2) {\n        ${b}\n        total += ${idx("j")};\n    }\n    total`,
  in_closure: (h, b, idx) =>
    `let mut f = |k: usize| -> u32 {\n        if k < ${h.len} {\n            ${b}\n            ${idx("k")}\n        } else {\n            0\n        }\n    };\n    f(k)`,
  // The index in a function of its own (`calleeBody`).
  in_callee: () => "",
};

/** Each program by name, `<holder>__<test>`. */
export function programs(): Record<string, string> {
  const all: Record<string, string> = {};
  for (const [holderName, h] of Object.entries(holders)) {
    for (const [testName, test] of Object.entries(tests)) {
      if (h.undefined?.includes(testName)) continue;
      const fns: string[] = [];
      const calls: string[] = [];
      for (const [betweenName, between] of Object.entries(betweens)) {
        const b = between(h);
        if (b === undefined) continue;
        for (const [formName, form] of Object.entries(forms)) {
          if (form(h, "k") === undefined || h.undefined?.includes(`${testName}/${formName}`)) continue;
          if ((testName === "in_callee" || testName === "wrap") && formName !== "item") continue;
          const name = `${betweenName}_${formName}`;
          const idx = (k: string) => form(h, k)!;
          const params = [h.params, "k: usize", "cut: &dyn Fn()"].filter(Boolean).join(", ");
          const callee = testName === "in_callee" ? `fn ${name}_at<T: Copy + Into<u32>>(s: &[T], k: usize) -> u32 {\n    s[k].into()\n}\n\n` : "";
          const body = testName === "in_callee" ? calleeBody(h, b, name) : test(h, b, idx, name);
          fns.push(`${callee}fn ${name}(${params}) -> u32 {\n    ${h.setup ?? ""}\n    ${body}\n}\n`);
          for (const k of [0, 2, 5]) {
            const args = [h.args, `${k}`, b.includes("cut") ? `&|| { ${h.cut}; }` : "&|| {}"].filter(Boolean).join(", ");
            calls.push(`    check("${name} ${k}", || {\n        ${h.make}\n        ${name}(${args})\n    });`);
          }
        }
      }
      all[`${holderName}__${testName}`] = `${decls}\n${fns.join("\n")}\nfn main() {\n${calls.join("\n")}\n}\n`;
    }
  }
  return { ...all, ...valuePrograms(), ...shapePrograms() };
}

/** The callee's test: the index in a function of its own, given the
 * collection's items as a slice, so its parameter's bound is its
 * callers' (caller-given parameters). */
function calleeBody(h: Holder, b: string, name: string): string {
  const slice = h.at("..");
  const of = slice.includes("with_borrow") ? undefined : `&${slice}`;
  if (of === undefined) {
    return `if k < ${h.len} {\n        ${b}\n        ITEMS.with_borrow(|v| ${name}_at(v, k))\n    } else {\n        0\n    }`;
  }
  return `if k < ${h.len} {\n        ${b}\n        ${name}_at(${of}, k)\n    } else {\n        0\n    }`;
}


// The value family: an integer tested, then written again through another
// path between, then used where its test decided the use: an array's index
// (a bounds assert), a sum that wraps past it, a cast that narrows it. Each
// write makes it 300, past each test.

/** Where an integer lives, as the probe sees it, and each way to write it
 * again. `read` reads it. */
type ValueHolder = {
  params: string;
  setup?: string;
  /** In `main`: statements making it `K`, and the probe's arguments but
   * `cut`. */
  make: string;
  args: string;
  read: string;
  /** A statement that sets it to `v`, as the probe sees it. */
  set: (v: string) => string;
  /** In `main`: a closure body that writes it through another path. */
  cut?: string;
  direct?: string;
  lend?: string;
  whole?: string;
  helper?: string;
  closure?: string;
  pointer?: string;
  /** As `Holder.undefined`. */
  undefined?: string[];
  /** The `<test>/<between>` programs rustc refuses, its constants folded
   * past the use: each left out. */
  refused?: string[];
};

const valueDecls = `use std::any::Any;
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check(what: &str, f: impl FnOnce() -> u32) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn bump(n: &mut u32) {
    *n = 300;
}

fn set_cell(n: &Cell<u32>) {
    n.set(300);
}

struct Counter {
    n: u32,
}

fn reset(c: &mut Counter) {
    c.n = 300;
}

thread_local! {
    static N: Cell<u32> = const { Cell::new(0) };
}
`;

const valueHolders: Record<string, ValueHolder> = {
  local: {
    params: "mut n: u32",
    make: "",
    args: "K",
    read: "n",
    set: (v) => `n = ${v};`,
    direct: "n += 296;",
    // `n = 2; n += 296;` is a constant rustc folds, and its index refused.
    refused: ["assigned/direct"],
    lend: "bump(&mut n);",
    closure: "let mut set = || n = 300; set();",
  },
  pointer: {
    params: "mut n: u32",
    setup: "let p: *mut u32 = &raw mut n;",
    make: "",
    args: "K",
    read: "n",
    set: (v) => `n = ${v};`,
    pointer: "unsafe { *p = 300 };",
    // Undefined behaviour, Miri's Stacked Borrows (nightly, 2026-11-15): the
    // closure's shared capture of `n` is invalidated by the write through
    // `p`, and `n` then read through it: "attempting a read access using
    // <33990> ..., but that tag does not exist in the borrow stack";
    // "<33990> was created by a SharedReadOnly retag" at the closure, "later
    // invalidated ... by a write access" at `*p = 300`.
    // And the tests that set `n` before the write through `p`: setting it
    // directly invalidates `p` ("attempting a write access using <34521>
    // ..., but that tag does not exist" at `*p = 300`).
    undefined: ["in_closure", "remainder", "assigned", "counted"],
  },
  mut_ref: {
    params: "n: &mut u32",
    make: "let mut v = K;",
    args: "&mut v",
    read: "*n",
    set: (v) => `*n = ${v};`,
    direct: "*n = 300;",
    lend: "bump(n);",
  },
  cell: {
    params: "n: &Cell<u32>",
    make: "let c = Cell::new(K);",
    args: "&c",
    read: "n.get()",
    set: (v) => `n.set(${v});`,
    cut: "c.set(300)",
    direct: "n.set(300);",
    helper: "set_cell(n);",
    closure: "let set = || n.set(300); set();",
  },
  rc_cell: {
    params: "n: Rc<Cell<u32>>",
    make: "let c = Rc::new(Cell::new(K)); let other = c.clone();",
    args: "c",
    read: "n.get()",
    set: (v) => `n.set(${v});`,
    cut: "other.set(300)",
    direct: "n.set(300);",
    helper: "set_cell(&n);",
  },
  field: {
    params: "c: &mut Counter",
    make: "let mut v = Counter { n: K };",
    args: "&mut v",
    read: "c.n",
    set: (v) => `c.n = ${v};`,
    direct: "c.n = 300;",
    lend: "bump(&mut c.n);",
    whole: "reset(c);",
  },
  thread_local: {
    params: "",
    make: "N.set(K);",
    args: "",
    read: "N.get()",
    set: (v) => `N.set(${v});`,
    cut: "N.set(300)",
    direct: "N.set(300);",
  },
};

const valueBetweens: Record<string, (h: ValueHolder) => string | undefined> = {
  nothing: () => "",
  callback: (h) => (h.cut ? "cut();" : undefined),
  direct: (h) => h.direct,
  lend: (h) => h.lend,
  whole: (h) => h.whole,
  helper: (h) => h.helper,
  closure: (h) => h.closure,
  pointer: (h) => h.pointer,
};

/** What the test decided: an index of an array of four, a sum that never
 * wraps below four, a cast to `u8` that keeps what's below four. */
const valueUses: Record<string, (x: string) => string> = {
  index: (x) => `[1u32, 2, 3, 4][${x} as usize]`,
  wrap: (x) => `${x} + 4294967292`,
  cast: (x) => `u32::from(${x} as u8)`,
};

const valueTests: Record<string, (h: ValueHolder, b: string, use: string) => string> = {
  below: (h, b, use) => `if ${h.read} < 4 {\n        ${b}\n        ${use}\n    } else {\n        0\n    }`,
  held: (h, b, use) => `let ok = ${h.read} < 4;\n    if ok {\n        ${b}\n        ${use}\n    } else {\n        0\n    }`,
  matched: (h, b, use) => `match ${h.read} {\n        0..=3 => {\n            ${b}\n            ${use}\n        }\n        _ => 0,\n    }`,
  // Set to a remainder below four, then used.
  remainder: (h, b, use) => `${h.set(`${h.read} % 4`)}\n    ${b}\n    ${use}`,
  // Set to a constant, then used.
  assigned: (h, b, use) => `${h.set("2")}\n    ${b}\n    ${use}`,
  // One less than what's at least one and below five.
  minus_one: (h, b, use) => `if ${h.read} >= 1 && ${h.read} < 5 {\n        ${b}\n        ${use.replace(h.read, `(${h.read} - 1)`)}\n    } else {\n        0\n    }`,
  // A counted loop, its count used past it.
  counted: (h, b, use) => `${h.set("0")}\n    while ${h.read} < 3 {\n        ${h.set(`${h.read} + 1`)}\n    }\n    ${b}\n    ${use}`,
  in_closure: (h, b, use) => `let mut f = || -> u32 {\n        if ${h.read} < 4 {\n            ${b}\n            ${use}\n        } else {\n            0\n        }\n    };\n    f()`,
};

/** The value family's programs, `value_<holder>__<test>`. */
function valuePrograms(): Record<string, string> {
  const all: Record<string, string> = {};
  for (const [holderName, h] of Object.entries(valueHolders)) {
    for (const [testName, test] of Object.entries(valueTests)) {
      if (h.undefined?.includes(testName)) continue;
      const fns: string[] = [];
      const calls: string[] = [];
      for (const [betweenName, between] of Object.entries(valueBetweens)) {
        const b = between(h);
        if (b === undefined || h.refused?.includes(`${testName}/${betweenName}`)) continue;
        for (const [useName, use] of Object.entries(valueUses)) {
          const name = `${betweenName}_${useName}`;
          const params = [h.params, "cut: &dyn Fn()"].filter(Boolean).join(", ");
          fns.push(`fn ${name}(${params}) -> u32 {\n    ${h.setup ?? ""}\n    ${test(h, b, use(h.read))}\n}\n`);
          for (const k of [0, 2, 5]) {
            const args = [h.args.replace("K", `${k}`), b.includes("cut") ? `&|| { ${h.cut}; }` : "&|| {}"].filter(Boolean).join(", ");
            calls.push(`    check("${name} ${k}", || {\n        ${h.make.replace("K", `${k}`)}\n        ${name}(${args})\n    });`);
          }
        }
      }
      all[`value_${holderName}__${testName}`] = `${valueDecls}\n${fns.join("\n")}\nfn main() {\n${calls.join("\n")}\n}\n`;
    }
  }
  return all;
}
