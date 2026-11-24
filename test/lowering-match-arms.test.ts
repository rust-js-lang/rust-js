// A match's arms: of more than one pattern an if chain, the value a match
// gives where no arm sets it given first, an or-pattern's arm that changes
// what it binds written once, matches binding the same names, and a match
// of constants a switch. Split from lowering-matches.test.ts by concept
// (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run } from "./support";

beforeAll(buildReact, 600_000);

// A match MIR tests as a tree, an arm reached from more than one of its
// leaves, `'0'..='9' | '.'`: an `if` chain of the arms, each tested where
// it's written, not a labeled block each leaf breaks out of.
test("a match's arms of more than one pattern are an if chain", async () => {
  const dir = fixture("decision-chain");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere(f64), Square(f64), Rect { w: f64, h: f64 }, Line { w: f64 } }
pub fn size(s: &Shape) -> f64 {
    match s {
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Square(x) => *x,
    }
}
pub fn shapes() -> Vec<f64> {
    [Shape::Circle(1.0), Shape::Sphere(2.0), Shape::Square(3.0), Shape::Rect { w: 4.0, h: 5.0 }, Shape::Line { w: 6.0 }]
        .iter()
        .map(size)
        .collect()
}
pub fn kind(c: char) -> u32 {
    match c {
        ' ' => 0,
        '0'..='9' | '.' => 1,
        'a'..='z' | 'A'..='Z' | '_' => 2,
        '+' | '-' => 3,
        _ => 4,
    }
}
pub fn log(c: char, out: &mut Vec<u32>) {
    match c {
        ' ' => out.push(0),
        '0'..='9' | '.' => out.push(1),
        'a'..='z' | 'A'..='Z' | '_' => {
            out.push(2);
            out.push(2)
        }
        '+' | '-' => out.push(3),
        _ => out.push(4),
    }
}
pub fn logged(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for c in s.chars() {
        log(c, &mut out);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect(js).toContain(`  if (s.TAG === "Circle" || s.TAG === "Sphere") {`);
  expect(js).toContain(`  if (s.TAG === "Rect" || s.TAG === "Line") {`);
  expect(js).toContain(`export function kind(c) {
  if (c === " ") {
    return 0;
  }
  if ((c >= "0" && c <= "9") || c === ".") {
    return 1;
  }
  if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_") {
    return 2;
  }
  if (c === "+" || c === "-") {
    return 3;
  }
  return 4;
}`);
  expect(js).toContain(`  } else if ((c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || c === "_") {
    out.push(2);
    out.push(2);
  } else if (c === "+" || c === "-") {`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.shapes()).toEqual([2, 4, 3, 4, 6]);
  const chars = [" ", "0", "5", "9", ".", "/", ":", "a", "z", "A", "Z", "_", "`", "{", "@", "[", "+", "-", "*", "é", "😀"];
  expect(chars.map((c) => lib.kind(c))).toEqual([0, 1, 1, 1, 1, 4, 4, 2, 2, 2, 2, 2, 4, 4, 4, 4, 3, 3, 4, 4, 4]);
  expect(lib.logged(chars.join(""))).toEqual([0, 1, 1, 1, 1, 4, 4, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 4, 4, 4, 4, 3, 3, 4, 4, 4]);
});

// A value a block gives where nothing else does, `matches!(v, Some(x) if
// ..)`'s `false`, given first: `let matched = false;`, then what sets it.
test("a value a match gives where no arm sets it is given first", async () => {
  const dir = fixture("default-value");
  writeFileSync(join(dir, "lib.rs"), `fn half(n: i32) -> Option<i32> {
    if n % 2 == 0 { Some(n / 2) } else { None }
}
pub fn chained(n: i32, calls: &mut u32) -> i32 {
    let v = if let Some(h) = half(n) && h != 0 && let Some(q) = { *calls += 1; half(h) } && q > 1 { q } else { 0 };
    v * 10 + *calls as i32
}
pub fn run() -> Vec<i32> {
    let mut calls = 0;
    [12, 4, 3, 0, 8].iter().map(|&n| chained(n, &mut calls)).collect()
}
fn fallback(calls: &mut u32) -> i32 {
    *calls += 100;
    -1
}
pub fn given(n: i32) -> (i32, u32) {
    let mut calls = 0;
    let v = match half(n) {
        Some(h) if h > 1 => h,
        _ => fallback(&mut calls),
    };
    (v, calls)
}
pub fn bumped(n: i32) -> (i32, i32) {
    let mut v = 5;
    let w = if let Some(h) = half(n) && { v += 1; h > v } { h } else { v };
    (w, v)
}
pub fn set_first(n: i32) -> i32 {
    let mut w = 3;
    #[allow(unused_assignments)]
    if let Some(h) = half(n) && { w = 7; h > 2 } { w = h; } else { w = 0; }
    w
}
pub fn read_first(n: i32) -> (i32, Vec<i32>) {
    let mut w = 3;
    let mut seen = Vec::new();
    if let Some(h) = half(n) && { seen.push(w); h > 2 } { w = h; } else { w = 0; }
    (w, seen)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const chained = js.slice(js.indexOf("export function chained"), js.indexOf("export function run"));
  expect(chained).not.toMatch(/\bbb\d+:/);
  expect(chained).toContain("  let v = 0;\n");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.run()).toEqual([31, 2, 2, 2, 23]);
  // A default of a call is made where no arm gives one, not first.
  expect([lib.given(8), lib.given(2), lib.given(3)]).toEqual([[4, 0], [-1, 100], [-1, 100]]);
  // Not of what isn't a constant, nor where the block reads or sets it first.
  expect([lib.bumped(4), lib.bumped(20)]).toEqual([[6, 6], [10, 6]]);
  expect([lib.setFirst(4), lib.setFirst(8), lib.setFirst(3)]).toEqual([0, 4, 0]);
  expect([lib.readFirst(4), lib.readFirst(8)]).toEqual([[0, [3]], [4, [3]]]);
});

// An or-pattern's arm that changes what it binds, `*r *= 10.0`, after the
// handle each alternative makes, of a field at its own key: written once,
// after the chain that makes it, not copied into each alternative's arm.
test("an or-pattern's arm that changes what it binds is written once", async () => {
  const dir = fixture("or-pattern-shared");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere { r: f64 }, Square(f64) }
pub fn grow(shapes: &mut [Shape]) {
    for s in shapes.iter_mut() {
        if let Shape::Circle(r) | Shape::Sphere { r } = s {
            *r *= 10.0;
        }
    }
}
pub fn grown() -> Vec<f64> {
    let mut shapes = vec![Shape::Circle(1.0), Shape::Square(2.0), Shape::Sphere { r: 3.0 }];
    grow(&mut shapes);
    shapes.iter().map(|s| match s { Shape::Circle(r) | Shape::Sphere { r } | Shape::Square(r) => *r }).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js.match(/r\.value \*= 10;/g)).toHaveLength(1);
  expect((await import(join(dir, "lib.js"))).grown()).toEqual([10, 2, 30]);
});

// Two matches whose arms bind the same names, `w` of each: each chain's
// variables its own, as JS declares a name once in a block.
test("matches binding the same names are chains of their own", async () => {
  const dir = fixture("chains-same-names");
  writeFileSync(join(dir, "lib.rs"), `pub enum Shape { Circle(f64), Sphere(f64), Square(f64), Rect { w: f64, h: f64 }, Line { w: f64 } }
pub fn both(a: &Shape, b: &Shape) -> f64 {
    let x = match a {
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Square(s) => *s,
    };
    let y = match b {
        Shape::Rect { w, .. } | Shape::Line { w } => *w,
        Shape::Circle(r) | Shape::Sphere(r) => r * 2.0,
        Shape::Square(s) => *s,
    };
    x + y
}
pub fn sums() -> Vec<f64> {
    vec![
        both(&Shape::Rect { w: 1.0, h: 9.0 }, &Shape::Line { w: 2.0 }),
        both(&Shape::Circle(1.0), &Shape::Square(5.0)),
        both(&Shape::Sphere(2.0), &Shape::Rect { w: 3.0, h: 0.0 }),
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+:/);
  expect((await import(join(dir, "lib.js"))).sums()).toEqual([3, 7, 7]);
});

// ADR 0629: a `match` the program wrote whose every arm tests constants is
// a `switch`, as React writes one: an or-pattern's constants stacked cases,
// a last `_` the `default`, left out where it does nothing; each case ends
// in `break;` where it'd go on, in a block where it declares; a `break` of
// the loop around it names the loop. Not one of two arms, an `if`'s two
// ways, nor one with a guard, nor one whose value is read in place, which
// JS has no `switch` expression for.
test("a match of constants is a switch", async () => {
  const dir = fixture("switches");
  writeFileSync(join(dir, "lib.rs"), `pub const UPPER: u8 = b'O';
const LOWER: u8 = b'c';
#[derive(Clone, Copy)]
pub enum Tag {
    Function,
    Class,
    Host,
}
pub fn kind(n: u32) -> &'static str {
    match n {
        0 => "zero",
        1 | 2 | 3 => "small",
        _ => "big",
    }
}
pub fn named(format: &str, i: usize) -> u32 {
    match format.as_bytes()[i] {
        UPPER | LOWER => 1,
        b'd' => 2,
        _ => 0,
    }
}
pub fn tag(t: Tag) -> u32 {
    match t {
        Tag::Function => 1,
        Tag::Class => 2,
        Tag::Host => 3,
    }
}
pub fn count(s: &str) -> u32 {
    let mut n = 0;
    for c in s.chars() {
        match c {
            'a' | 'e' => n += 1,
            'x' => break,
            'y' => continue,
            _ => {}
        }
        n += 10;
    }
    n
}
pub fn rows(grid: &[&str]) -> u32 {
    let mut n = 0;
    'rows: for row in grid {
        for c in row.chars() {
            match c {
                '#' => break 'rows,
                '.' => break,
                _ => n += 1,
            }
        }
    }
    n
}
pub fn declares(n: u32, log: &mut Vec<String>) {
    match n {
        1 => {
            let s = format!("one{n}");
            log.push(s);
        }
        2 => log.push("two".to_string()),
        _ => {}
    }
}
pub fn guarded(n: u32) -> u32 {
    match n {
        1 if n > 0 => 1,
        2 => 2,
        _ => 3,
    }
}
pub fn length(s: &str) -> u32 {
    let len = s.len();
    let label = match len {
        0 => 10,
        1 => 20,
        _ => 30,
    };
    label + match s.len() {
        0 => 1,
        1 => 2,
        _ => 3,
    }
}
pub fn spoken(n: u32) -> String {
    format!("{}!", match n { 1 => "one", 2 => "two", _ => "many" })
}
pub fn two(n: u32) -> &'static str {
    match n {
        0 => "zero",
        _ => "other",
    }
}
pub fn skips(xs: &[u32]) -> u32 {
    let mut n = 0;
    for x in xs {
        match x {
            1 => n += 1,
            2 => n += 2,
            _ => continue,
        }
    }
    n
}
pub fn sized(s: &str) -> u32 {
    match s.len() {
        0 => 1,
        1 | 2 => 2,
        _ => 3,
    }
}
pub fn slices(n: u32) -> usize {
    let slice: &[u8] = match n {
        0 => &[1],
        1 => &[2, 3],
        _ => &[],
    };
    slice.len()
}

pub fn specifier(code: u8, log: &mut Vec<u8>) {
    match code {
        b'O' | b'c' | b'd' => log.push(code),
        _ => {}
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Of one arm of three constants and a `_`, a case of each.
  expect(js).toContain("  switch (code) {\n    case 79:\n    case 99:\n    case 100:\n      log.push(code);\n");
  expect(js).toContain(
    '  switch (n) {\n    case 0:\n      return "zero";\n    case 1:\n    case 2:\n    case 3:\n      return "small";\n    default:\n      return "big";\n  }',
  );
  // What's matched read in place, its temporary's value; a `const` by its name.
  expect(js).toContain("  switch (bytes[i]) {\n    case UPPER:\n    case LOWER:\n      return 1;\n    case 100:");
  // Each variant a case, the last too: Rust tested every one.
  expect(js).toContain('    case "Class":\n      return 2;\n    case "Host":\n      return 3;\n  }');
  // An arm that breaks the loop, which goes on past it: an `if` chain (#389).
  expect(js).toContain(
    '  for (const c of s) {\n    if (c === "a" || c === "e") {\n      n = (n + 1) >>> 0;\n    } else if (c === "x") {\n      break;\n    } else if (c === "y") {\n      continue;\n    }\n',
  );
  // The program's label; a turn that ends at the `switch` left after it.
  expect(js).toContain('          break rows;\n        case ".":\n          break;\n        default:\n          n = (n + 1) >>> 0;\n          continue;\n      }\n      break;\n');
  expect(js).toContain("    case 1: {\n      const s = `one${n}`;\n      log.push(s);\n      break;\n    }\n");
  expect(js).not.toMatch(/default:\n *break;/);
  expect(js).toContain("  if (n === 1 && n > 0) {\n    return 1;\n  }\n  if (n === 2) {");
  // A variable the program named, by its name; a value read in place, its `?:`.
  expect(js).toContain("  switch (len) {\n");
  expect(js).toContain("  switch ($byteLen(s)) {\n");
  expect(js).toContain("  let slice;\n  switch (n) {\n    case 0:\n      slice = [1];\n");
  expect(js).toContain("$byteLen(s) === 0 ? 1 : $byteLen(s) === 1 ? 2 : 3");
  expect(js).toContain('n === 1 ? "one" : n === 2 ? "two" : "many"');
  // Of two arms, the \`if\` of two ways: a value's conditional (ADR 0237).
  expect(js).toContain('  return n === 0 ? "zero" : "other";\n');
  const lib = await import(join(dir, "lib.js"));
  expect([0, 2, 9].map(lib.kind)).toEqual(["zero", "small", "big"]);
  expect([0, 1, 2, 3].map((i) => lib.named("Ocdz", i))).toEqual([1, 1, 2, 0]);
  expect(["Function", "Class", "Host"].map(lib.tag)).toEqual([1, 2, 3]);
  expect([lib.count("abeyz"), lib.count("aaxa")]).toEqual([42, 22]);
  expect(lib.rows(["ab.c", "d#e", "fg"])).toEqual(3);
  const log: string[] = [];
  [1, 2, 3].forEach((n) => lib.declares(n, log));
  expect(log).toEqual(["one1", "two"]);
  expect([1, 2, 5].map(lib.guarded)).toEqual([1, 2, 3]);
  expect(["", "a", "abc"].map(lib.length)).toEqual([11, 22, 33]);
  expect([1, 2, 3].map(lib.spoken)).toEqual(["one!", "two!", "many!"]);
  expect([0, 1].map(lib.two)).toEqual(["zero", "other"]);
  expect(lib.skips([1, 2, 5, 2])).toEqual(5);
  expect(["", "ab", "abc"].map(lib.sized)).toEqual([1, 2, 3]);
  expect([0, 1, 2].map(lib.slices)).toEqual([1, 2, 0]);
  const pushed: number[] = [];
  [79, 1, 100].forEach((code) => lib.specifier(code, pushed));
  expect(pushed).toEqual([79, 100]);
});
