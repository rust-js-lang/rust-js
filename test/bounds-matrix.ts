// The bounds matrix (ADR 1137, the audit before step 5's switch): a
// collection's length tested, then something run that may change it,
// then an index of it, each holder of the collection meeting each test,
// each thing between and each form of index, that rust-js must run as
// native Rust does. What the MIR intervals prove of a length is sound
// only while nothing between can change it: another path to the same
// collection (another `Rc`, a shared `&RefCell`, a field), a callback, a
// closure, a reassignment, a `&mut` lent, a drop. Each program is one
// holder and one test, a function for each thing between and each form.

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

// The shape family: each rule the audit's ablation found proving corpus
// sites but no matrix row, its own shape, a control its rule proves and
// each write that would break the rule run against native (rustc-tests.yml
// `ablation`'s gaps). Where nothing between can break the proof, a
// quotient's or a live slice's length sum, the control alone holds the rule.

const shapeDecls = `use std::any::Any;
use std::cell::{Cell, RefCell};
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
  // wraps. Not past a write through a \`&mut\` of it.
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
  // A generic constant bounds its loop: `i += 1` below \`N\` never wraps.
  // Not past a write through a \`&mut\` of the counter.
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
  // A counter down from a length: \`a[s - 1]\` under \`s > 0\`, its relation
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
  // What a value is below bounds its interval: \`v[i >> 1]\` under
  // \`i < v.len()\`. Not of the slice made another between.
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
  // \`vec![x; v.len()]\` is as long as \`v\`. Not past a pop of it.
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
  // A slice's \`get(i)\`'s \`Some\` puts \`i\` below its length: \`i += 1\` never
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
};

/** The shape family's programs, `shape_<name>`. */
function shapePrograms(): Record<string, string> {
  return Object.fromEntries(
    Object.entries(shapes).map(([name, s]) => [`shape_${name}`, `${shapeDecls}\n${s.fns}\nfn main() {\n${s.calls.map((c) => `    ${c};`).join("\n")}\n}\n`]),
  );
}
