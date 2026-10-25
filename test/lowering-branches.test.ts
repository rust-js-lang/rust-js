// What the compiler writes of a program's branches and returns: an `if` as a
// person writes it, a conditional of two values, `&&` and `||` chains, an
// `else` kept or left out, and a return where it's written. Each test is a
// program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// An `else` of nothing, no code and no comment, is no `else`. One of only
// comments is kept, with them (ADR 0753, `test/lowering-comments.test.ts`).
test("an else of nothing is left out", async () => {
  const dir = fixture("empty-else");
  writeFileSync(join(dir, "lib.rs"), `pub fn picked(flag: bool, out: &mut Vec<u32>) {
    if flag {
        out.push(1);
    } else {
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  if (flag) {\n    out.push(1);\n  }\n}");
  const out: number[] = [];
  (await import(join(dir, "lib.js"))).picked(true, out);
  expect(out).toEqual([1]);
});

// ADR 0364: what MIR makes of a branch reads as a person writes it:
// `a && b`, `a || b`, `matches!`'s `n === "x"`, `c ? a : b` of two values,
// and no check of a constant divisor.
test("branches from MIR read as a person writes them", async () => {
  const dir = fixture("mir-branches");
  writeFileSync(join(dir, "lib.rs"), `pub fn both(a: u32, b: u32) -> bool {
    a > 1 && b > 2
}
pub fn either(a: u32, b: u32) -> bool {
    a > 1 || b > 2
}
pub fn named(n: Option<&str>) -> bool {
    matches!(n, Some("x"))
}
pub fn sign(n: i32) -> &'static str {
    let s = if n < 0 { "-" } else { "+" };
    s
}
pub fn odd(i: u32) -> bool {
    i % 2 == 1
}
pub fn flags(home: bool, index: bool, section: &str) -> (bool, bool) {
    let mut toc = true;
    let mut cols = true;
    if home || index {
        toc = false;
    } else if section == "blog" {
        toc = false;
        cols = false;
    }
    (toc, cols)
}
pub fn guard(a: u32, s: &str) -> bool {
    if a > 1 { s == "x" } else { false }
}
pub fn matched(a: Option<u32>) -> bool {
    match a {
        Some(n) => n > 2,
        None => false,
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return a > 1 && b > 2;");
  expect(js).toContain("return a > 1 || b > 2;");
  expect(js).toContain('return n === "x";');
  expect(js).toContain('n < 0 ? "-" : "+"');
  expect(js).not.toContain("2 === 0");
  expect(js).toContain('  if (home || index) {\n    toc = false;\n  } else if (section === "blog") {\n');
  // A branch of `false` the program wrote with an `if`: the conditional
  // it wrote, not the `&&` MIR makes the same branches of (ADR 0879).
  expect(js).toContain('return a > 1 ? s === "x" : false;');
  // One MIR made of a `match`'s arm of `false`: the `&&` a person writes.
  expect(js).toContain("return a != null && a > 2;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.both(2, 3), lib.either(0, 1), lib.named("x"), lib.named(undefined), lib.sign(-1), lib.odd(3), lib.flags(false, false, "blog")]).toEqual([true, false, true, false, "-", true, [false, false]]);
});

// A function that returns nothing, from MIR (ADR 0364), returns where the
// program wrote \`return\` alone: what falls off its end needs none, and an
// \`if\` MIR made a guard of is the \`if\` the program wrote.
test("a function returning nothing returns where the program wrote it", async () => {
  const dir = fixture("mir-unit-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn explicit(x: bool, v: &mut Vec<u32>) {
    if x {
        return;
    }
    v.push(1);
}
pub fn nested(a: Option<u32>, b: bool, out: &mut Vec<u32>) {
    if let Some(x) = a {
        if b {
            out.push(x);
        }
    }
}
pub fn guarded(flag: bool, cur: Option<&str>, out: &mut Vec<String>) {
    if flag && let Some(c) = cur {
        if c.len() > 2 {
            out.push(c.to_string());
        }
    }
}
pub fn one(n: u32, v: &mut Vec<u32>) {
    match n {
        1 => v.push(1),
        _ => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function explicit(x, v) {
  if (x) {
    return;
  }
  v.push(1);
}
export function nested(a, b, out) {
  if (a != null) {
    if (b) {
      out.push(a);
    }
  }
}
export function guarded(flag, cur, out) {
  if (flag && cur != null) {
    if ($byteLen(cur) > 2) {
      out.push(cur);
    }
  }
}
export function one(n, v) {
  if (n === 1) {
    v.push(1);
  }
}
`);
  const lib = await import(join(dir, "mir.js"));
  const [v, w, x]: unknown[][] = [[], [], []];
  lib.explicit(false, v);
  lib.nested(3, true, w);
  lib.guarded(true, "abc", x);
  expect([v, w, x]).toEqual([[1], [3], ["abc"]]);
});

// An `if` whose branch leaves has no `else`: what follows it runs only when
// the branch doesn't, as JS writes it and react.dev's Link has it, `if (..)
// { return cloneElement(..); } return child;` (ADR 0237). A branch that
// doesn't leave keeps its `else`.
test("an if whose branch returns has no else", async () => {
  const dir = fixture("no-else-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn classify(n: i32) -> &'static str {
    if n < 0 {
        println!("neg");
        "neg"
    } else if n == 0 {
        println!("zero");
        "zero"
    } else {
        println!("pos");
        "pos"
    }
}
// A branch that leaves by an inner \`if\` both of whose branches do.
pub fn nested(a: bool, b: bool) -> u32 {
    if a {
        if b {
            println!("ab");
            1
        } else {
            println!("a");
            2
        }
    } else {
        println!("none");
        3
    }
}
pub fn counted(n: i32) -> i32 {
    let mut total = 0;
    if n > 0 {
        total += n;
    } else {
        total -= n;
    }
    total
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  if (n < 0) {\n    console.log("neg");\n    return "neg";\n  }\n  if (n === 0) {\n    console.log("zero");\n    return "zero";\n  }\n  console.log("pos");\n  return "pos";\n}');
  expect(js).toContain("total = n > 0 ? total + n : (total - n) | 0;");
  expect(js).toContain('    console.log("a");\n    return 2;\n  }\n  console.log("none");\n  return 3;\n}');
  const lib = await import(join(dir, "lib.js"));
  const log = console.log;
  console.log = () => {};
  try {
    expect([lib.classify(-1), lib.classify(0), lib.classify(3), lib.counted(2), lib.counted(-2)]).toEqual(["neg", "zero", "pos", 2, 2]);
    expect([lib.nested(true, true), lib.nested(true, false), lib.nested(false, true)]).toEqual([1, 2, 3]);
  } finally {
    console.log = log;
  }
});

// One whose arms bind what their subject holds, named where it is, is a
// conditional too (ADR 0209), as react.dev's DocsFooter picks a link or a
// `<div />`. What needs a `const` of its own is statements, as before.
test("a two-arm match whose arms bind places is a conditional expression", async () => {
  const dir = fixture("match-conditional-binds");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub title: String,
    pub path: Option<String>,
}
pub fn path(item: Option<&Item>) -> &str {
    let label = match item {
        Some(Item { path: Some(path), .. }) => path.as_str(),
        _ => "none",
    };
    label
}
// The second arm's too.
pub fn either(r: Result<u32, u32>) -> u32 {
    let v = match r {
        Ok(n) => n,
        Err(n) => n + 1,
    };
    v
}
// A guard reads what its arm binds.
pub fn titled(item: Option<&Item>) -> &str {
    let label = match item {
        Some(item) if item.path.is_some() => item.title.as_str(),
        _ => "untitled",
    };
    label
}
// What's owned has a \`const\` of its own, dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn owned(some: bool) -> u32 {
    let l = if some { Some(Loud(7)) } else { None };
    let n = match l {
        Some(l) => l.0,
        None => 0,
    };
    println!("after");
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const label = item && item.path != null ? item.path : "none";');
  expect(js).toContain('const v = r.TAG === "Ok" ? r._0 : (r._0 + 1) >>> 0;');
  expect(js).toContain('const label = item && item.path != null ? item.title : "untitled";');
  expect(js).not.toContain("let tmp");
  const lib = await import(join(dir, "lib.js"));
  const item = { title: "T", path: "/p" };
  expect([lib.path(item), lib.path({ title: "T" }), lib.path(undefined)]).toEqual(["/p", "none", "none"]);
  expect([lib.either({ TAG: "Ok", _0: 1 }), lib.either({ TAG: "Err", _0: 1 })]).toEqual([1, 2]);
  expect([lib.titled(item), lib.titled({ title: "T" })]).toEqual(["T", "untitled"]);
  const logs: string[] = [];
  const log = console.log;
  console.log = (line: string) => logs.push(line);
  try {
    expect([lib.owned(true), lib.owned(false)]).toEqual([7, 0]);
  } finally {
    console.log = log;
  }
  expect(logs).toEqual(["drop 7", "after", "after"]);
});

// A two-arm `match` as a value, its arms plain and binding nothing, is a
// conditional, as a person writes it (ADR 0209): its subject in place
// where the test reads it once, else in a `const` of its own.
test("a two-arm match that's a value is a conditional expression", async () => {
  const dir = fixture("match-conditional");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Primary,
    Secondary,
}
pub fn button_class(kind: Option<Kind>) -> &'static str {
    let class = match kind.unwrap_or(Kind::Primary) {
        Kind::Primary => "bg-link",
        Kind::Secondary => "text-primary",
    };
    class
}
pub fn low(x: u32) -> &'static str {
    let size = match x % 3 {
        0 | 1 => "low",
        _ => "high",
    };
    size
}
// A plain guard is the arm's test too.
pub fn guarded(n: u32, flag: bool) -> &'static str {
    let label = match n {
        0 if flag => "flagged zero",
        _ => "other",
    };
    label
}
// A &mut to a number, a cell, is tested by its value.
pub fn zero(n: &mut i32) -> &'static str {
    let label = match n {
        &mut 0 => "zero",
        _ => "other",
    };
    label
}
// A bool tested against true is the bool, as JS has it.
pub fn emptiness(xs: Vec<u32>) -> &'static str {
    let label = match xs.is_empty() {
        true => "empty",
        false => "some",
    };
    label
}
pub fn toggle(flag: bool) -> &'static str {
    let label = match flag {
        false => "off",
        true => "on",
    };
    label
}
// A first arm that takes everything is the value, its guard the test.
#[allow(unreachable_patterns)]
pub fn every(n: u32, flag: bool) -> &'static str {
    let all = match n {
        _ => "all",
        1 => "one",
    };
    let guarded = match n {
        _ if flag => "flag",
        _ => "no flag",
    };
    if all == "all" { guarded } else { "never" }
}
// A subject with a destructor is dropped where Rust drops it.
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
pub fn dropped() -> &'static str {
    let label = match Loud(1) {
        Loud(1) => "one",
        _ => "other",
    };
    println!("after");
    label
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('(kind ?? "Primary") === "Primary" ? "bg-link" : "text-primary"');
  // Of three cases, `0 | 1`'s two and `_`, a switch (ADR 0629).
  expect(js).toContain('  let size;\n  switch (x % 3) {\n    case 0:\n    case 1:\n      size = "low";\n      break;\n    default:\n      size = "high";\n  }\n');
  expect(js).toContain('const label = n === 0 && flag ? "flagged zero" : "other";');
  expect(js).toContain('const label = n.value === 0 ? "zero" : "other";');
  expect(js).toContain('const label = xs.length === 0 ? "empty" : "some";');
  expect(js).toContain('const label = !flag ? "off" : "on";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.buttonClass(undefined), lib.buttonClass("Secondary"), lib.low(4), lib.low(5)]).toEqual(["bg-link", "text-primary", "low", "high"]);
  expect([lib.guarded(0, true), lib.guarded(0, false), lib.guarded(1, true)]).toEqual(["flagged zero", "other", "other"]);
  expect([lib.zero({ value: 0 }), lib.zero({ value: 3 })]).toEqual(["zero", "other"]);
  expect([lib.every(1, true), lib.every(1, false)]).toEqual(["flag", "no flag"]);
  expect([lib.emptiness([]), lib.emptiness([1])]).toEqual(["empty", "some"]);
  expect([lib.toggle(false), lib.toggle(true)]).toEqual(["off", "on"]);
  const logged: string[] = [];
  const log = console.log;
  console.log = (line: string) => logged.push(line);
  try {
    expect(lib.dropped()).toBe("one");
  } finally {
    console.log = log;
  }
  expect(logged).toEqual(["drop 1", "after"]);
});

// ADR 0599: a function's value its branches make before it's coerced, a
// `Box<dyn Fn()>`'s, is returned where each is made, as any other is.
test("branches' values coerced to the result are returned where made", async () => {
  const dir = fixture("coerced-branches");
  writeFileSync(join(dir, "lib.rs"), `pub fn pick(now: bool) -> Box<dyn Fn() -> u32> {
    if now {
        Box::new(|| 1)
    } else {
        let n = 2;
        Box::new(move || n)
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function pick(now) {\n  if (now) {\n    return () => 1;\n  }\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.pick(true)(), lib.pick(false)()]).toEqual([1, 2]);
});

// ADR 0613: \`&&\` and \`||\` mixed, as the program wrote them, are one value
// returned, \`(x === y && (x !== 0 || 1 / x === 1 / y)) || ..\`, React's
// \`Object.is\` polyfill's, not MIR's labelled block of its tests.
test("a value of && and || mixed is returned as written", async () => {
  const dir = fixture("mixed-logicals");
  writeFileSync(join(dir, "lib.rs"), `pub fn same(x: f64, y: f64) -> bool {
    (x == y && (x != 0.0 || 1.0 / x == 1.0 / y)) || (x.is_nan() && y.is_nan())
}

pub fn either(a: bool, b: bool, c: bool, d: bool) -> bool {
    (a || b) && (c || d)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  return (x === y && (x !== 0 || 1 / x === 1 / y)) || (Number.isNaN(x) && Number.isNaN(y));\n");
  expect(js).not.toContain("break");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.same(0, -0), lib.same(NaN, NaN), lib.same(1, 1), lib.same(1, 2)]).toEqual([false, true, true, false]);
  const all = [false, true];
  for (const a of all) for (const b of all) for (const c of all) for (const d of all) {
    expect(lib.either(a, b, c, d)).toBe((a || b) && (c || d));
  }
});

// ADR 0299: a `return` of `undefined` is a bare `return;`, as react.dev's
// NavigationBar ends its effect, `} else { return; }`.
test("a return of undefined is a bare return", async () => {
  const dir = fixture("bare-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn halved(n: u32) -> Option<u32> {
    if n % 2 == 1 {
        return None;
    }
    Some(n / 2)
}

pub fn cleanup(on: bool) -> Option<fn()> {
    fn noop() {}
    if on { Some(noop) } else { None }
}

pub fn effect(on: bool) -> impl Fn() -> Option<u32> {
    move || {
        if on {
            println!("on");
            return Some(1);
        }
        None
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("return undefined;");
  // One at a function's end says nothing: it falls off its end there, a
  // closure's too, as a person ends one.
  expect(js.match(/return;/g)?.length).toBe(1);
  expect(js).toContain("  if (on) {\n    return noop;\n  }\n}\n");
  expect(js).toContain("      return 1;\n    }\n  };\n}\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.halved(3), lib.halved(4), lib.cleanup(false), lib.effect(false)(), lib.effect(true)()]).toEqual([undefined, 2, undefined, undefined, 1]);
});

// ADR 0309: a match that only gives a value is one expression, as a person
// writes it, as the playground's app has `current`, `text` and `file`: an
// option's value or a default, `e || d` of an object, `e ?? d` of text,
// which may be `""` (ADR 0298, amended); a part of it never null or
// undefined, `e?.[1] ?? d`; otherwise `m ? f(m) : d`, of a `const m`. And
// `map_or` is the same expression.
test("a match that only gives a value is one expression", async () => {
  const dir = fixture("value-matches");
  writeFileSync(join(dir, "lib.rs"), `pub struct State {
    pub n: u32,
}

fn find(states: &[State], n: u32) -> Option<&State> {
    states.iter().find(|s| s.n == n)
}

pub fn picked<'a>(states: &'a [State], blank: &'a State) -> u32 {
    let current = match find(states, 1) {
        Some(state) => state,
        None => blank,
    };
    current.n + current.n
}

pub fn shown_text(files: &[(String, String)], shown: &str) -> String {
    let text = match files.iter().find(|(path, _)| path == shown) {
        Some((_, text)) => text.clone(),
        None => String::new(),
    };
    format!("{text}{text}")
}

pub fn after(path: &str) -> u32 {
    let n = match path.find('x') {
        Some(i) => i as u32 + 1,
        None => 0,
    };
    n * n
}

pub fn mapped(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, file)| file)
}

pub fn field_of(state: Option<&State>) -> u32 {
    let n = match state {
        Some(s) => s.n,
        None => 0,
    };
    n * n
}

fn pair_of(n: u32) -> Option<(u32, Option<u32>)> {
    match n {
        0 => None,
        1 => Some((1, None)),
        _ => Some((n, Some(n))),
    }
}

pub fn inner(n: u32) -> u32 {
    let inner = match pair_of(n) {
        Some((_, inner)) => inner,
        None => Some(7),
    };
    inner.unwrap_or(0) + inner.unwrap_or(0)
}

pub fn named<'a>(names: &[&'a str], blank: &'a str) -> String {
    let name = match names.iter().copied().find(|s| s.len() < 2) {
        Some(name) => name,
        None => blank,
    };
    format!("{name}{name}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  const current = state || blank;\n");
  expect(js).toMatch(/  const name = name\$1 \?\? blank;\n/);
  expect(js).toContain('  const text = find ? find[1] : "";\n');
  expect(js).toMatch(/  const i = \$find\(path, "x"\);\n  const n = i != null \? \(i \+ 1\) >>> 0 : 0;\n/);
  expect(js).toContain('  return $rsplitOnce(path, "/")?.[1] ?? path;\n');
  // Of a variable, as `map_or` of it.
  expect(js).toContain("  const n = state ? state.n : 0;\n");
  const lib = await import(join(dir, "lib.js"));
  const states = [{ n: 1 }, { n: 2 }];
  expect([lib.picked(states, { n: 9 }), lib.picked([], { n: 9 })]).toEqual([2, 18]);
  expect([lib.shownText([["a", "A"], ["b", "B"]], "b"), lib.shownText([], "b")]).toEqual(["BB", ""]);
  expect([lib.after("abxd"), lib.after("ab"), lib.mapped("a/b/c"), lib.mapped("c")]).toEqual([9, 0, "c", "c"]);
  // A part that may be `None` itself isn't `?? d`'s: `Some((1, None))` gives `None`.
  expect([lib.inner(0), lib.inner(1), lib.inner(3)]).toEqual([14, 0, 6]);
  expect([lib.fieldOf({ n: 3 }), lib.fieldOf(undefined)]).toEqual([9, 0]);
  expect([lib.named(["abc", ""], "x"), lib.named(["abc"], "x")]).toEqual(["", "xx"]);
});

// A function's `if` of an `||` as its value: one conditional, the `||` its
// one test; an `&&` written as the value, one value returned.
test("an if of a logical is one conditional", async () => {
  const dir = fixture("if-logical-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn prec(op: char) -> u8 {
    if op == '+' || op == '-' { 1 } else { 2 }
}

pub fn both(a: u8, b: u8) -> bool {
    a > 1 && b > 1
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return op === "+" || op === "-" ? 1 : 2;\n');
  expect(js).toContain("return a > 1 && b > 1;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.prec("+"), lib.prec("*"), lib.both(2, 2), lib.both(2, 0)]).toEqual([1, 2, true, false]);
});

// A function's tail `if` of two small values is one conditional, as a
// variable's is, React's scheduler's `return diff !== 0 ? diff : a.id - b.id`
// (ADR 0575); it was each branch returned, rust-js's form of react.dev's.
test("a tail if of two small values is one conditional", async () => {
  const dir = fixture("tail-if-returns");
  writeFileSync(join(dir, "lib.rs"), `pub struct Email(pub String);

pub fn email(s: String) -> Result<Email, String> {
    if s.contains('@') { Ok(Email(s)) } else { Err(format!("{s} is not an email")) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain('  return s.includes("@") ? { TAG: "Ok", _0: [s] } : { TAG: "Err", _0: `${s} is not an email` };');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.email("a@b"), lib.email("x")]).toEqual([{ TAG: "Ok", _0: ["a@b"] }, { TAG: "Err", _0: "x is not an email" }]);
});

// Each branch adding to a variable, `out += a`: the branches, as written,
// not `out = c ? out + a : out + b`.
test("branches that each add to a variable stay branches", async () => {
  const dir = fixture("branch-updates");
  writeFileSync(join(dir, "lib.rs"), `pub fn report(results: Vec<Result<u32, String>>) -> String {
    let mut out = String::new();
    for r in results {
        match r {
            Ok(n) => out += &format!("ok {n}\\n"),
            Err(e) => out += &format!("err {e}\\n"),
        }
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("out += `ok ${");
  expect(js).toContain("out += `err ${");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.report([{ TAG: "Ok", _0: 1 }, { TAG: "Err", _0: "x" }])).toBe("ok 1\nerr x\n");
});

// A function's value of a \`match\`'s arms is returned from each arm; its
// \`if\`/\`else\` of two values is one conditional, as a variable's is.
test("a function's match returns each arm, and its if of two values is one conditional", async () => {
  const dir = fixture("mir-returned-branches");
  writeFileSync(join(dir, "lib.rs"), `pub fn two(s: &str) -> u32 {
    match s {
        "a" => 1,
        _ => 2,
    }
}
pub fn cond(s: &str) -> u32 {
    if s == "a" { 1 } else { 2 }
}
pub fn bools(b: bool) -> u32 {
    match b {
        true => 1,
        false => 2,
    }
}
pub fn opt(o: Option<u32>) -> u32 {
    match o {
        Some(x) => x,
        None => 0,
    }
}
pub fn early(g: bool, x: u32) -> bool {
    if !g {
        return false;
    }
    x > 1
}
pub fn any(a: bool, b: bool) -> bool {
    a || b
}
pub fn shown(o: Option<u32>) -> String {
    match o {
        Some(_) => format!("#{}", 1),
        None => String::new(),
    }
}
pub struct F { pub n: u32 }
pub fn either(o: Option<&F>, r: bool) -> bool {
    o.is_some() || r
}
pub fn colors(message: &str) -> &'static str {
    match message {
        "warn" => "yellow",
        "error" => "red",
        _ => "gray",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function two(s) {
  if (s === "a") {
    return 1;
  }
  return 2;
}`);
  expect(js).toContain(`export function cond(s) {
  return s === "a" ? 1 : 2;
}`);
  expect(js).toContain(`export function bools(b) {
  if (b) {
    return 1;
  }
  return 2;
}`);
  expect(js).toContain(`export function opt(o) {
  if (o != null) {
    return o;
  }
  return 0;
}`);
  // Of three arms, a \`switch\` (ADR 0629).
  expect(js).toContain(`export function colors(message) {
  switch (message) {
    case "warn":
      return "yellow";
    case "error":
      return "red";
    default:
      return "gray";
  }
}`);
  expect(js).toContain("  return a || b;\n");
  // A `return` the program wrote stays where it wrote it.
  expect(js).toContain("export function early(g, x) {\n  if (!g) {\n    return false;\n  }\n  return x > 1;\n}");
  // An arm a macro begins is in its place too.
  expect(js).toContain("export function shown(o) {\n  if (o != null) {\n");
  // A `bool` of what's not one, `!!o`.
  expect(js).toContain("  return !!o || r;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.either({ n: 1 }, false), lib.either(undefined, false)]).toEqual([true, false]);
});

test("an `if !c` from MIR, which MIR tests as `c`, keeps its branch first", async () => {
  const dir = fixture("mir-negated-if");
  writeFileSync(join(dir, "lib.rs"), `pub fn guard(active: bool, v: &mut Vec<u32>) {
    if !active {
        v.push(0);
        return;
    }
    v.push(1);
    v.push(2);
}
pub fn either(active: bool) -> u32 {
    if !active { 1 } else { 2 }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function guard(active, v) {
  if (!active) {
    v.push(0);
    return;
  }
  v.push(1);
  v.push(2);
}`);
  expect(js).toContain(`export function either(active) {
  return !active ? 1 : 2;
}`);
});

test("tests and values from MIR read as a person writes them: De Morgan's, and `filter` then `or`", async () => {
  const dir = fixture("mir-tests-values");
  writeFileSync(join(dir, "lib.rs"), `pub fn search(section: &str, is_blog_index: bool, order: u32) -> Option<u32> {
    let mut search_order = None;
    if section == "learn" || (section == "blog" && !is_blog_index) {
        search_order = Some(order);
    }
    search_order
}
pub struct Meta { pub title: Option<&'static str> }
pub fn title(meta: &Meta, route: Option<&'static str>) -> &'static str {
    meta.title.filter(|t| !t.is_empty()).or(route.filter(|t| !t.is_empty())).unwrap_or("")
}
pub struct Route { pub title: Option<&'static str> }
pub fn routed(route: Option<&Route>) -> &'static str {
    route.and_then(|r| r.title).filter(|t| !t.is_empty()).unwrap_or("")
}
pub fn page_title(meta: &Meta, route: Option<&Route>) -> &'static str {
    meta.title.filter(|t| !t.is_empty()).or(route.and_then(|r| r.title).filter(|t| !t.is_empty())).unwrap_or("")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (section === "learn" || (section === "blog" && !isBlogIndex)) {\n');
  expect(js).toContain('  return meta.title || route || "";\n');
  expect(js).toContain('  return route?.title || "";\n');
  expect(js).toContain('  return meta.title || route?.title || "";\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.search("learn", true, 1), lib.search("blog", true, 2), lib.search("blog", false, 3), lib.search("x", false, 4)]).toEqual([1, undefined, 3, undefined]);
  expect([lib.title({ title: "" }, "r"), lib.title({ title: "t" }, "r"), lib.title({ title: undefined }, ""), lib.title({}, undefined)]).toEqual(["r", "t", "", ""]);
});

test("an `if … else if` given as a value from MIR is a conditional where it's read", async () => {
  const dir = fixture("mir-else-if-value");
  writeFileSync(join(dir, "lib.rs"), `pub fn link(href: &str) -> String {
    format!(
        "[{}]",
        if href.starts_with("https://") {
            "ext"
        } else if href.starts_with('#') {
            "hash"
        } else {
            href
        }
    )
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return `[${href.startsWith("https://") ? "ext" : href.startsWith("#") ? "hash" : href}]`;\n');
});

test("a function's value from MIR that branches assign is returned from each, its guards first", async () => {
  const dir = fixture("mir-assigned-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn footer(href: Option<&str>, children: Vec<String>) -> String {
    let classes = format!("c{}", children.len());
    let Some(href) = href.filter(|href| !href.is_empty()) else {
        return format!("<div {classes}>");
    };
    if href.starts_with("https://") {
        return format!("<ext {href} {classes}>");
    }
    format!("<next {href} {classes}>")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function footer(href, children) {
  const classes = \`c\${children.length}\`;
  if (!href) {
    return \`<div \${classes}>\`;
  }
  if (href.startsWith("https://")) {
    return \`<ext \${href} \${classes}>\`;
  }
  return \`<next \${href} \${classes}>\`;
}`);
});

// A match of three arms or more giving a variable its value, from MIR, is
// the chain of them: not its last two folded, \`c ? a : b\`.
test("a variable's value of three arms from MIR is their chain", async () => {
  const dir = fixture("mir-three-arms");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Kind { Note, Warn { level: u8 }, Tip(&'static str) }
pub fn class(k: Kind) -> String {
    let cls = match k {
        Kind::Note => "note",
        Kind::Warn { .. } => "warn",
        Kind::Tip(_) => "tip",
    };
    format!("{cls}!")
}
// Of a value MIR holds, \`tmp\`, its first branch doing more than giving
// it, as react.dev's CodeDiagram picks a child.
pub fn pick(x: i32) -> String {
    let shown: Box<dyn std::fmt::Display> = if x > 5 {
        let m = x * 2;
        Box::new(m + 1)
    } else if x < 0 {
        Box::new("neg")
    } else {
        Box::new(x)
    };
    shown.to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (k === "Note") {\n    cls = "note";\n  } else if (k.TAG === "Warn") {\n    cls = "warn";\n  } else {\n    cls = "tip";\n  }\n');
  expect(js).toMatch(/} else if \(x < 0\) {\n    \w+ = /);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.class$("Note"), lib.class$({ TAG: "Warn", level: 1 }), lib.class$({ TAG: "Tip", _0: "x" })]).toEqual(["note!", "warn!", "tip!"]);
  expect([lib.pick(6), lib.pick(-1), lib.pick(2)]).toEqual(["13", "neg", "2"]);
});

test("a test from MIR that `x` is set and true reads `x`", async () => {
  const dir = fixture("mir-set-and-true");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    Start { first_load: Option<bool> },
    Refresh,
}
pub fn starts(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
}
pub fn titled(error_code: Option<&str>) -> String {
    match error_code {
        Some(code) if !code.is_empty() => format!("#{code}"),
        _ => "none".to_string(),
    }
}
// A field set and true, \`props.meta\`, and one after another test.
pub struct Props {
    pub meta: Option<String>,
}
pub fn meta(props: &Props) -> usize {
    let mut n = 0;
    if let Some(meta) = &props.meta
        && !meta.is_empty()
    {
        n = meta.len();
    }
    n
}
pub fn logs(message: &Message, out: &mut Vec<u32>) {
    if let Message::Start { first_load } = message
        && *first_load == Some(true)
    {
        out.push(2);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("!= null &&");
  expect(js).toContain("  if (props.meta) {\n");
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.starts({ TAG: "Start", firstLoad: true }, pushed);
  lib.starts({ TAG: "Start", firstLoad: false }, pushed);
  lib.starts({ TAG: "Start" }, pushed);
  lib.starts("Refresh", pushed);
  expect([pushed, lib.titled("e"), lib.titled(""), lib.titled(undefined)]).toEqual([[1, 1], "#e", "none", "none"]);
  lib.logs({ TAG: "Start", firstLoad: true }, pushed);
  lib.logs({ TAG: "Start" }, pushed);
  expect([pushed, lib.meta({ meta: "ab" }), lib.meta({ meta: "" }), lib.meta({})]).toEqual([[1, 1, 2], 2, 0, 0]);
});

test("an `||` of `matches!` from MIR is one test, at a function's end or not", async () => {
  const dir = fixture("mir-or-matches");
  writeFileSync(join(dir, "lib.rs"), `pub enum Message {
    Start { first_load: Option<bool> },
    Refresh,
    Other,
}
pub fn starts(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
    out.push(2);
}
pub fn tail(message: &Message, out: &mut Vec<u32>) {
    if matches!(message, Message::Start { first_load: Some(true) }) || matches!(message, Message::Refresh) {
        out.push(1);
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const test = '  if ((message.TAG === "Start" && message.firstLoad) || message === "Refresh") {\n    out.push(1);\n  }\n';
  expect(js).toContain(`export function starts(message, out) {\n${test}  out.push(2);\n}`);
  expect(js).toContain(`export function tail(message, out) {\n${test}}`);
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.starts({ TAG: "Start", firstLoad: true }, pushed);
  lib.tail("Refresh", pushed);
  lib.tail("Other", pushed);
  expect(pushed).toEqual([1, 2, 1]);
});

// A guard of what a variant binds, `Some(code) if !code.is_empty()` of an
// `as_deref()`: `errorCode`, its truth, as react.dev's error page tests it,
// not `errorCode != null && errorCode` and a `const code = errorCode;`.
test("a guarded binding of an Option's string is its truth", async () => {
  const dir = fixture("guarded-string");
  writeFileSync(join(dir, "lib.rs"), `pub fn title(error_code: Option<String>) -> String {
    match error_code.as_deref() {
        Some(code) if !code.is_empty() => format!("Minified React error #{code}"),
        _ => "Minified Error Decoder".to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // MIR's one value, as react.dev's original writes it.
  expect(js).toContain(
    'export function title(errorCode) {\n  return errorCode ? `Minified React error #${errorCode}` : "Minified Error Decoder";\n}',
  );
  const lib = await import(join(dir, "mir.js"));
  expect([lib.title("1"), lib.title(""), lib.title(undefined)]).toEqual(["Minified React error #1", "Minified Error Decoder", "Minified Error Decoder"]);
});

// Text's truth, `!!code` of `!code.is_empty()`, isn't a presence test: an
// empty string is there. `code ? code.toUpperCase() : undefined` is not
// `code?.toUpperCase()`, which gives `""` where Rust gives `None`; an
// `Option` never falsy's `!!o` is one still (ADR 0298).
test("a test of text's truth is no optional chain", async () => {
  const dir = fixture("text-truth-chain");
  writeFileSync(join(dir, "lib.rs"), `pub fn held(code: Option<&str>) -> Option<String> {
    let s = match code {
        Some(code) if !code.is_empty() => Some(code.to_uppercase()),
        _ => None,
    };
    s
}
pub fn upper(s: &str) -> Option<String> {
    let upper = if !s.is_empty() { Some(s.to_uppercase()) } else { None };
    upper
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("?.toUpperCase()");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.held(""), lib.held("a"), lib.held(undefined), lib.upper(""), lib.upper("b")]).toEqual([undefined, "A", undefined, undefined, "B"]);
});

// A let chain's tests, each read where it's tested once what holds them
// is gone, one `if`: `val != null` of `Array.isArray(val)` none, as
// react.dev's errors page revives its elements.
test("a let chain from MIR is one test, past what held its parts", async () => {
  const dir = fixture("let-chain-tests");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown, classify};

pub fn revive(val: Option<&'static Unknown>) -> Option<&'static Unknown> {
    if let Some(val) = val
        && matches!(classify(Some(val)), Kind::Array(_))
        && matches!(classify(js::get(val, &0)), Kind::String("$r"))
    {
        return js::get(val, &1);
    }
    val
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('export function revive(val) {\n  if (Array.isArray(val) && val[0] === "$r") {\n    return val[1];\n  }\n  return val;\n}');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.revive(["$r", 1]), lib.revive(["x", 1]), lib.revive(undefined), lib.revive("$r")]).toEqual([1, ["x", 1], undefined, "$r"]);
});

// An `||` of `&&`s, `(a && b) || (c && (d || e)) || f`, which MIR jumps
// out of where each holds, is the one test written, as react.dev's Search
// opens on its keys.
test("an or of ands from MIR is one test", async () => {
  const dir = fixture("or-of-ands");
  writeFileSync(join(dir, "lib.rs"), `pub struct Ev {
    pub key_code: u32,
    pub key: String,
    pub meta: bool,
    pub ctrl: bool,
}
fn editing(ev: &Ev) -> bool {
    ev.key == "x"
}
pub fn handle(ev: &Ev, open: bool, out: &mut Vec<u32>) {
    if (ev.key_code == 27 && open) || (ev.key == "k" && (ev.meta || ev.ctrl)) || (!editing(ev) && ev.key == "/" && !open) {
        out.push(1);
        if open {
            out.push(2);
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (\n    (ev.keyCode === 27 && open) ||\n    (ev.key === "k" && (ev.meta || ev.ctrl)) ||\n    (!editing(ev) && ev.key === "/" && !open)\n  ) {\n    out.push(1);\n');
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  for (const [ev, open] of [[{ keyCode: 27, key: "", meta: false, ctrl: false }, true], [{ keyCode: 0, key: "k", meta: false, ctrl: true }, false], [{ keyCode: 0, key: "k", meta: false, ctrl: false }, false], [{ keyCode: 0, key: "/", meta: false, ctrl: false }, false], [{ keyCode: 0, key: "/", meta: false, ctrl: false }, true]] as const) lib.handle(ev, open, pushed);
  expect(pushed).toEqual([1, 2, 1, 1]);
});

// Returns the program wrote, which MIR's lowering makes assignments of the
// result and jumps out of a block to its one `return`, from a loop's too:
// each a `return` where it's written, as react.dev's getRouteMeta finds a
// route's breadcrumbs.
test("written returns MIR jumps out of a block for are returns where they're written", async () => {
  const dir = fixture("block-returns");
  writeFileSync(join(dir, "lib.rs"), `pub struct Route {
    pub path: Option<String>,
    pub routes: Option<Vec<Route>>,
}
pub fn crumbs<'a>(path: &str, current: &'a Route, crumbs_so_far: Vec<&'a Route>) -> Vec<&'a Route> {
    if current.path.as_deref() == Some(path) {
        return crumbs_so_far;
    }
    let Some(routes) = &current.routes else {
        return Vec::new();
    };
    for route in routes {
        let child = crumbs(path, route, [crumbs_so_far.as_slice(), &[current]].concat());
        if !child.is_empty() {
            return child;
        }
    }
    Vec::new()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    if (child.length !== 0) {\n      return child;\n    }\n  }\n  return [];\n}");
  const lib = await import(join(dir, "mir.js"));
  const tree = { path: "/", routes: [{ path: "/a", routes: [{ path: "/a/b" }] }] };
  expect([lib.crumbs("/a/b", tree, []).map((r: { path: string }) => r.path), lib.crumbs("/x", tree, [])]).toEqual([["/", "/a"], []]);
});

// An `if` of an `||`, its branches as the program wrote them: `home ||
// !meta.title ? a : b`, not `!home && meta.title ? b : a`, as react.dev's
// Page picks its image.
test("an if of an or from MIR keeps its branches in the order written", async () => {
  const dir = fixture("or-branch-order");
  writeFileSync(join(dir, "lib.rs"), `pub struct Meta {
    pub title: &'static str,
    pub path: &'static str,
}
pub fn image(home: bool, meta: &Meta) -> String {
    let image = if home || meta.title.is_empty() {
        format!("/images/og-{}.png", meta.path)
    } else {
        format!("/images/og/{}.png", meta.path.replace('/', "-"))
    };
    image
}
// A test's truth, not two values: as written.
pub fn flagged(heading: bool, crumb: bool, expanded: bool) -> Option<&'static str> {
    (!heading && crumb && !expanded).then_some("on")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    home || !meta.title\n      ? `/images/og-${meta.path}.png`\n");
  expect(js).toContain('  return !heading && crumb && !expanded ? "on" : undefined;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.image(true, { title: "t", path: "/a" }), lib.image(false, { title: "t", path: "/a/b" }), lib.flagged(false, true, false)]).toEqual(["/images/og-/a.png", "/images/og/-a-b.png", "on"]);
});

// An `||` chain, `a || b || c`, not `!(!a && !b) || c`: a `bool` field,
// `element.editable`, is one, as react.dev's Search, Preview and
// useTocHighlight test theirs.
test("an or chain from MIR is written as one", async () => {
  const dir = fixture("or-chain");
  writeFileSync(join(dir, "lib.rs"), `pub struct El {
    pub editable: bool,
    pub tag: String,
}
pub fn editing(element: &El) -> bool {
    let tag_name = &element.tag;
    element.editable || tag_name == "INPUT" || tag_name == "SELECT" || tag_name == "TEXTAREA"
}
pub fn hidden(error: Option<u32>, height: Option<f64>, ready: bool) -> bool {
    let hide = error.is_some() || height.is_none_or(|h| h == 0.0) || !ready;
    hide
}
pub fn heading(names: Vec<String>) -> Vec<String> {
    names.into_iter().filter(|n| n.as_str() == "H1" || n.as_str() == "H2" || n.as_str() == "H3").collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return element.editable || tagName === "INPUT" || tagName === "SELECT" || tagName === "TEXTAREA";');
  expect(js).toContain("const hide = error != null || height == null || height === 0 || !ready;");
  expect(js).toContain('names.filter((n) => n === "H1" || n === "H2" || n === "H3")');
  const lib = await import(join(dir, "mir.js"));
  expect([
    lib.editing({ editable: false, tag: "SELECT" }),
    lib.editing({ editable: false, tag: "B" }),
    lib.hidden(undefined, 3, true),
    lib.hidden(undefined, 0, true),
    lib.heading(["H2", "P"]),
  ]).toEqual([true, false, false, true, ["H2"]]);
});

// An `if let .. else if let .. else { panic!() }` chain is its arms in
// the order written, not the panic's first in a labeled block, as
// react.dev's TerminalBlock reads its message.
test("an if-let chain ending in a panic keeps its arms in written order", async () => {
  const dir = fixture("if-let-chain-panic");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Text(String),
    Element(Option<String>),
    Other,
}
pub fn message(kind: Kind) -> String {
    let message: String;
    if let Kind::Text(text) = &kind {
        message = text.to_string();
    } else if let Kind::Element(child) = &kind
        && let Some(text) = child
    {
        message = text.to_string();
    } else {
        panic!("Expected a plain string.");
    }
    message
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (kind.TAG === "Text") {\n    message = kind._0;\n  } else if (kind.TAG === "Element" && kind._0 != null) {\n    message = kind._0;\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message({ TAG: "Text", _0: "a" }), lib.message({ TAG: "Element", _0: "b" })]).toEqual(["a", "b"]);
  expect(() => lib.message({ TAG: "Element", _0: undefined })).toThrow("plain string");
});

// A binding a let chain makes in its condition, kept a `const` in its
// `then`, is the `then`'s: the chain is still an `else if`, no label to
// reach its `else` (ADR 0659).
test("an if-let chain's binding kept in its arm keeps the chain an else if", async () => {
  const dir = fixture("if-let-chain-binding");
  writeFileSync(join(dir, "lib.rs"), `pub enum Kind {
    Text(String),
    Element(Vec<String>),
    Other,
}
pub fn message(kind: Kind) -> String {
    let message: String;
    if let Kind::Text(text) = &kind {
        message = text.to_string();
    } else if let Kind::Element(child) = &kind
        && let Some(text) = child.first()
        && text.len() > 1
    {
        message = text.clone();
    } else {
        panic!("Expected a plain string.");
    }
    message
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  if (kind.TAG === "Text") {
    message = kind._0;
  } else if (kind.TAG === "Element" && kind._0[0] != null && $byteLen(kind._0[0]) > 1) {
    const text = kind._0[0];
    message = text;
  } else {
    throw new Error("Expected a plain string.");
  }`);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.message({ TAG: "Text", _0: "a" }), lib.message({ TAG: "Element", _0: ["bc"] })]).toEqual(["a", "bc"]);
  expect(() => lib.message({ TAG: "Element", _0: ["b"] })).toThrow("plain string");
  expect(() => lib.message({ TAG: "Other" })).toThrow("plain string");
});

// An `||` chain whose part is an `Option`'s `is_some()`, of an object:
// `!!error || ..`, a `bool` part by part, not `!(!error && ..)`; and one
// only tested, `error || ..`, as react.dev's Preview hides its content.
test("an or chain of a value's truth from MIR is a bool part by part", async () => {
  const dir = fixture("or-chain-truth");
  writeFileSync(join(dir, "lib.rs"), `pub struct Err {
    pub title: String,
}
pub fn hidden(error: Option<&Err>, height: Option<f64>, ready: bool) -> bool {
    error.is_some() || height.is_none_or(|h| h == 0.0) || !ready
}
pub fn kept(error: Option<&Err>, height: Option<f64>, ready: bool) -> String {
    let hide = error.is_some() || height.is_none_or(|h| h == 0.0) || !ready;
    if hide { "a".to_string() } else { "b".to_string() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return !!error || height == null || height === 0 || !ready;");
  expect(js).toContain("const hide = error || height == null || height === 0 || !ready;");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.hidden({ title: "t" }, 3, true), lib.hidden(undefined, 3, true), lib.kept({ title: "t" }, 3, true), lib.kept(undefined, 3, true)]).toEqual([true, false, "a", "b"]);
});

// An `else if` chain's last arm before a panic is an `else if`, not a
// check in the `else`, `if (x !== "language-css") throw`, as react.dev's
// createFileMap picks a code block's file.
test("an else-if chain's arm before a panic stays an else if", async () => {
  const dir = fixture("else-if-panic");
  writeFileSync(join(dir, "lib.rs"), `pub fn path(meta: Option<&str>, class_name: &str) -> String {
    let file_path;
    if let Some(meta) = meta {
        file_path = format!("/{meta}");
    } else if class_name == "language-js" {
        file_path = "/App.js".to_string();
    } else if class_name == "language-css" {
        file_path = "/styles.css".to_string();
    } else {
        panic!("Code block is missing a filename: {class_name}");
    }
    file_path
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  } else if (className === "language-css") {\n    filePath = "/styles.css";\n  } else {\n    throw');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.path(undefined, "language-css"), lib.path("a.js", "x")]).toEqual(["/styles.css", "/a.js"]);
  expect(() => lib.path(undefined, "x")).toThrow("missing a filename");
});

// An `||` chain whose first test is a getter's `bool`, `element.isContentEditable`,
// is one chain returned, as react.dev's Search tells whether a key goes to
// what's being edited.
test("an or chain of a getter's bool is returned as one", async () => {
  const dir = fixture("getter-or-chain");
  writeFileSync(join(dir, "lib.rs"), `use react::webapi::HTMLElement;
pub fn editing(element: &HTMLElement) -> bool {
    let tag_name = element.tag_name();
    element.is_content_editable() || tag_name == "INPUT" || tag_name == "SELECT" || tag_name == "TEXTAREA"
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return (\n    element.isContentEditable ||\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.editing({ isContentEditable: false, tagName: "SELECT" }), lib.editing({ isContentEditable: false, tagName: "B" })]).toEqual([true, false]);
});

// A call of a function that gives a `bool`, `isItem(item)`, is one, so
// it's no `!!isItem(item)` in a test, as react.dev's CodeDiagram keeps its
// images.
test("a call giving a bool is one in a test", async () => {
  const dir = fixture("bool-call");
  writeFileSync(join(dir, "lib.rs"), `pub struct Item {
    pub kind: String,
}
#[cfg_attr(rust_js, rust_js::link_name = "globalThis.isItem")]
#[allow(unused_variables)]
fn is_item(x: &Item) -> bool {
    unreachable!()
}
pub fn images(items: Vec<&Item>) -> Vec<&Item> {
    items.into_iter().filter(|item| is_item(item) && item.kind == "img").collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('items.filter((item) => globalThis.isItem(item) && item.kind === "img")');
  const lib = await import(join(dir, "mir.js"));
  (globalThis as any).isItem = (x: { kind: string }) => x.kind !== "";
  expect(lib.images([{ kind: "img" }, { kind: "p" }, { kind: "" }])).toEqual([{ kind: "img" }]);
});

// A value the program wrote with `&&`, `s != "en" && (if t { .. } else {
// .. })`, is one value returned, `s !== "en" && (t ? .. : ..)`, not a
// `return` of each branch MIR makes of it, as react.dev's MDXComponents
// filters its LanguageList; a tail `if` of two values is one conditional.
test("a value written with && is returned as one", async () => {
  const dir = fixture("logical-values");
  writeFileSync(join(dir, "lib.rs"), `pub fn a(t: bool, x: u32) -> u32 {
    if t { x + 1 } else { x * 2 }
}
pub fn c(t: bool, u: bool, s: &str) -> bool {
    s != "en" && (if t { s.is_empty() } else { u })
}
pub fn kept(all: &[String], complete: bool, done: &[&str]) -> usize {
    all.iter()
        .filter(|code| *code != "en" && (if complete { done.contains(&code.as_str()) } else { !done.contains(&code.as_str()) }))
        .count()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return s !== "en" && (t ? !s : u);');
  expect(js).toContain('(code) => code !== "en" && (complete ? done.includes(code) : !done.includes(code))');
  expect(js).toContain("  return t ? (x + 1) >>> 0 : (x * 2) >>> 0;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.c(true, false, ""), lib.c(false, true, "en"), lib.kept(["en", "fr", "de"], true, ["fr"])]).toEqual([true, false, 1]);
});

// An `if` the program wrote with an `else` keeps it, in a loop too, where
// MIR's structure makes a guard of an `else` that returns, `if (!c) {
// return; } …`, and a `continue` of a branch the loop's body ends with;
// its `else if` too, React's scheduler's `siftDown` (ADR 0575). A guard
// the program wrote, an `if` with no `else`, stays one.
test("an if written with an else keeps it, in a loop and of an else-if", async () => {
  const dir = fixture("written-elses");
  writeFileSync(join(dir, "lib.rs"), `pub fn sift_down(heap: &mut Vec<f64>, i: usize) -> usize {
    let mut index = i;
    let length = heap.len();
    while index < length / 2 {
        let left = index * 2 + 1;
        let right = left + 1;
        if heap[left] < heap[index] {
            if right < length && heap[right] < heap[left] {
                heap.swap(index, right);
                index = right;
            } else {
                heap.swap(index, left);
                index = left;
            }
        } else if right < length && heap[right] < heap[index] {
            heap.swap(index, right);
            index = right;
        } else {
            // Neither child is smaller.
            return index;
        }
    }
    index
}

pub fn compare(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let diff = a - b;
    if diff != 0.0 { diff } else { c - d }
}

pub fn clamped(n: u32) -> u32 {
    if n > 10 {
        return 10;
    }
    n
}

pub fn named(give: bool, name: String) -> Option<String> {
    if give { Some(name) } else { None }
}

pub fn first_big(items: &[u32]) -> u32 {
    for &item in items {
        if item > 10 {
            return item;
        }
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each index read is below the length once it's read (ADR 0881): its
  // swap JS's own.
  expect(js).toContain("      } else {\n        [heap[index], heap[left]] = [heap[left], heap[index]];");
  expect(js).toContain("    } else if (right < length && ");
  expect(js).toContain("    } else {\n      // Neither child is smaller.\n      return index;\n    }\n  }");
  expect(js).not.toContain("continue;");
  expect(js).toContain("    if (item > 10) {\n      return item;\n    }");
  // A tail `if` of two small values, each path's return: one conditional.
  expect(js).toContain("  return diff !== 0 ? diff : c - d;");
  // A guard at the function's end, and a branch that gives nothing: as written.
  expect(js).toContain("  if (n > 10) {\n    return 10;\n  }\n  return n;");
  expect(js).toContain("  if (give) {\n    return name;\n  }");
  const lib = await import(join(dir, "lib.js"));
  const heap = [5, 1, 2, 3];
  expect([lib.siftDown(heap, 0), heap, lib.firstBig([1, 20, 30]), lib.firstBig([1])]).toEqual([3, [1, 3, 2, 5], 20, 0]);
});

// ADR 0657: a value an `if` gives, each branch one expression, a function
// too, is `c ? a : b`, of a module's variable as of a function's `let`, as
// React's `reportGlobalError` is `typeof reportError === 'function' ?
// reportError : error => { .. }`; a branch that does something before its
// value, or a chain of more, stays an `if`.
test("an if of one value each, a function's too, is a conditional", async () => {
  const dir = fixture("conditional-functions");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsFast")]
fn fast(n: u32) -> u32 {
    unreachable!()
}

thread_local! {
    static PICK: fn(u32) -> u32 = if js::is_defined(fast) { fast } else { |n| n * 2 };
}

pub fn picked(n: u32) -> u32 {
    PICK.with(|pick| pick(n))
}

pub fn local(flag: bool, n: u32) -> u32 {
    let f: fn(u32) -> u32 = if flag { |n| n + 1 } else { |n| n * 3 };
    f(n) + f(1)
}

pub fn done(flag: bool, n: u32) -> u32 {
    let f: fn(u32) -> u32 = if flag {
        println!("one");
        |n| n + 1
    } else {
        |n| n * 3
    };
    f(n)
}

pub fn chained(n: u32) -> fn(u32) -> u32 {
    if n == 0 {
        |n| n + 1
    } else if n == 1 {
        |n| n + 2
    } else {
        |n| n + 3
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const pick = typeof rustJsFast !== "undefined" ? rustJsFast : (n) => (n * 2) >>> 0;');
  expect(js).toContain("  const f = flag ? (n) => (n + 1) >>> 0 : (n) => (n * 3) >>> 0;\n");
  expect(js).toContain('  let f;\n  if (flag) {\n    console.log("one");\n');
  expect(js).toContain("  if (n === 0) {\n    return (n) => (n + 1) >>> 0;\n  }\n");
  const globals = globalThis as { rustJsFast?: (n: number) => number };
  const lib = await import(join(dir, "lib.js"));
  expect([lib.picked(4), lib.local(true, 2), lib.local(false, 2), lib.chained(1)(1)]).toEqual([8, 5, 9, 3]);
  expect(globals.rustJsFast).toBeUndefined();
});
