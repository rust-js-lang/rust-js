// What the compiler writes of a program's loops: `for…of` and counted `for`
// from MIR, `while`, `break` and `continue` where they're written, labels, and
// an `else if` at a loop's end. Each test is a program of its own, compiled
// and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0628: a loop the program wrote after another is after it, as written,
// not in the first one's way out, where MIR puts it: React's
// normalizeConsoleFormat walks its format, then pads it.
test("a loop after a loop is after it", async () => {
  const dir = fixture("loop-after-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn padded(text: &str, n: usize) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        out += "a";
        i += 1;
    }
    let mut j = 0;
    while j < n {
        out += "b";
        j += 1;
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  while (i < $byteLen(text)) {\n");
  expect(js).toContain("  }\n  let j = 0;\n  while (j < n) {\n");
  expect(js).not.toContain("while (true)");
  expect((await import(join(dir, "lib.js"))).padded("xy", 3)).toBe("aabbb");
});

// A `for` loop from MIR (ADR 0364) is the `for…of` or counted `for` a
// person writes, not the `while (true)` stepping an iterator MIR has.
test("loops from MIR read as a person writes them", async () => {
  const dir = fixture("mir-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(items: &[u32]) -> u32 {
    let mut n = 0;
    for x in items {
        if *x > 2 {
            n += 1;
        }
    }
    n
}
pub fn found(items: &[u32]) -> Option<u32> {
    for &x in items {
        if x > 2 {
            return Some(x);
        }
    }
    None
}
pub fn pairs(items: &[(u32, u32)]) -> u32 {
    let mut n = 0;
    for (a, b) in items {
        n += a * b;
    }
    n
}
pub fn upto(k: u32) -> u32 {
    let mut n = 0;
    for i in 0..k {
        n += i;
    }
    n
}
pub fn stops(items: &[u32]) -> u32 {
    let mut n = 0;
    for x in items {
        if *x == 0 {
            break;
        }
        n += x;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const x of items) {\n    if (x > 2) {\n      n = ");
  expect(js).toContain("  for (const x of items) {\n    if (x > 2) {\n      return x;\n");
  expect(js).toContain("  for (const [a, b] of items) {\n");
  expect(js).toContain("  for (let i = 0; i < k; i++) {\n");
  expect(js).toContain("    if (x === 0) {\n      break;\n    }\n");
  expect(js).not.toContain("while (true)");
  expect(js).not.toContain("$iter");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted([1, 3, 4]), lib.found([1, 3]), lib.found([1]), lib.pairs([[1, 2], [3, 4]]), lib.upto(4), lib.stops([1, 2, 0, 5])]).toEqual([2, 3, undefined, 14, 6, 3]);
});

// `if let`s chained with `&&`, and loops, from MIR (ADR 0364), as written:
// one condition's tests one `&&`, a variant's binding the place it reads
// where that reads the same, a tuple's parts in place, a loop's labeled
// block its `continue`; and `if`s the program nests, nested.
test("if-let chains and loops from MIR read as written", async () => {
  const dir = fixture("mir-let-chains");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
pub struct Route { pub title: Option<String>, pub path: Option<String>, pub routes: Option<Vec<Route>> }
pub fn pages(routes: &[Route]) -> Vec<String> {
    let mut out = Vec::new();
    for route in routes {
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            out.push(format!("{title}:{}", children.len()));
        } else if let (Some(title), Some(path)) = (&route.title, &route.path) {
            out.push(format!("{title}={path}"));
        }
    }
    out
}
pub fn sections(routes: &[Route]) -> Vec<String> {
    let mut out = Vec::new();
    for route in routes {
        if route.path.as_deref().is_some_and(|p| p.starts_with("http")) {
            continue;
        }
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            for child in children {
                if let Some(t) = &child.title {
                    out.push(format!("{title}/{t}"));
                }
            }
            continue;
        }
        if let Some(path) = &route.path {
            out.push(path.clone());
        }
    }
    out
}
pub fn nested(a: Option<u32>, b: bool) -> u32 {
    let mut n = 0;
    if let Some(x) = a {
        if b {
            n = x;
        }
    }
    n
}
// A closure's, by its own THIR.
pub fn joined(routes: &[Route]) -> Vec<String> {
    routes
        .iter()
        .map(|r| {
            if let Some(t) = &r.title
                && let Some(p) = &r.path
            {
                format!("{t}{p}")
            } else {
                String::new()
            }
        })
        .collect()
}
pub struct Item { pub kind: &'static str, pub valid: bool }
fn valid(i: &Item) -> Option<&Item> {
    if i.valid { Some(i) } else { None }
}
pub fn kept(items: &[Item]) -> usize {
    items.iter().filter(|c| !matches!(valid(c), Some(e) if e.kind == "pre")).count()
}
// Data whose fields may change, which a value is read out of first, or a
// reference to a field of, which reads the field where it's used.
pub struct Named { pub title: Option<String>, pub seen: Cell<bool> }
pub fn titled(nodes: &[Named]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        node.seen.set(true);
        if let Some(title) = &node.title
            && title.len() > 1
        {
            out.push(title.clone());
        }
    }
    out
}
pub struct Node { pub count: Option<u32>, pub items: Vec<u32>, pub seen: Cell<bool> }
pub fn counts(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        if node.seen.get()
            && let Some(c) = node.count
            && c > 1
        {
            n += c;
        } else {
            n += 1;
        }
    }
    n
}
pub fn totals(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        node.seen.set(true);
        for item in &node.items {
            n += item;
        }
    }
    n
}
pub fn chained(nodes: &[Node]) -> u32 {
    let mut n = 0;
    for node in nodes {
        if node.seen.get()
            && let Some(c) = node.count
        {
            for item in &node.items {
                n += item * c;
            }
            continue;
        }
        n += 1;
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  for (const route of routes) {
    if (route.title != null && route.routes && route.routes.length !== 0) {
      out.push(\`\${route.title}:\${route.routes.length}\`);
    } else if (route.title != null && route.path != null) {
      out.push(\`\${route.title}=\${route.path}\`);
    }
  }
`);
  expect(js).toContain(`  for (const route of routes) {
    if (route.path != null && route.path.startsWith("http")) {
      continue;
    }
    if (route.title != null && route.routes && route.routes.length !== 0) {
      for (const child of route.routes) {
        if (child.title != null) {
          out.push(\`\${route.title}/\${child.title}\`);
        }
      }
      continue;
    }
    if (route.path != null) {
      out.push(route.path);
    }
  }
`);
  expect(js).toContain("  if (a != null) {\n    if (b) {\n");
  expect(js).toContain("    if (node.seen && node.count != null && node.count > 1) {\n");
  expect(js).not.toContain("let iter");
  expect(js).not.toMatch(/bb\d+:/);
  expect(js).toMatch(/r\.title != null && r\.path != null/);
  expect(js).toContain("    if (node.title != null && $byteLen(node.title) > 1) {\n      out.push(node.title);\n");
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { title: "a", path: "x", routes: [{ title: "c" }] },
    { title: "b", path: "y", routes: [] },
    { path: "http://z" },
  ];
  expect([lib.pages(routes), lib.sections(routes), lib.nested(3, true), lib.nested(3, false)]).toEqual([["a:1", "b=y"], ["a/c", "y"], 3, 0]);
  const nodes = () => [{ count: 3, items: [1, 2], seen: true }, { count: undefined, items: [5], seen: false }];
  expect([lib.counts(nodes()), lib.totals(nodes()), lib.chained(nodes())]).toEqual([4, 8, 10]);
  expect(lib.joined(routes)).toEqual(["ax", "by", ""]);
  expect(lib.titled([{ title: "ab", seen: false }, { title: "a", seen: false }])).toEqual(["ab"]);
  expect(lib.kept([{ kind: "pre", valid: true }, { kind: "pre", valid: false }, { kind: "p", valid: true }])).toBe(2);
});

// What a loop's recovery (ADR 0364) no longer shows: an iterator stepped by
// hand, a std one and a generic one given an array, each a JS iterator; a
// \`None\` of a \`Duration\` tested \`== null\`, as \`ZERO\` is falsy; and a
// \`const\` another takes is that one, from MIR.
test("iterators stepped by hand, a Duration's None and a const another takes", async () => {
  const dir = fixture("mir-by-hand");
  writeFileSync(join(dir, "lib.rs"), `use std::time::Duration;
pub struct B { pub a: u32, pub c: u32 }
pub fn moved() -> u32 {
    let whole = B { a: 5, c: 6 };
    let mut moved = whole;
    moved.c = 7;
    moved.a + moved.c
}
pub fn second(s: &str) -> Option<u8> {
    let mut b = s.bytes();
    b.next();
    b.next()
}
fn first<I: IntoIterator<Item = u32>>(items: I) -> Option<u32> {
    items.into_iter().next()
}
pub fn first_of(v: Vec<u32>) -> Option<u32> {
    first(v)
}
pub fn zero(a: u64, b: u64) -> bool {
    let d = Duration::from_secs(a).checked_sub(Duration::from_secs(b));
    if let Some(x) = d { x.as_secs() == 0 } else { false }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const moved = { a: 5, c: 6 };\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.moved(), lib.second("ab"), lib.firstOf([4, 5]), lib.zero(5n, 5n), lib.zero(4n, 5n)]).toEqual([12, 98, 4, true, false]);
});

// Loops one after another from MIR (ADR 0364), the second in what the
// first leaves to, each item named as the program names it: each a
// \`for…of\`, the innermost first; a map's \`insert\` nothing reads, \`set\`;
// and a \`let\` the loops held, a \`const\`.
test("loops one after another from MIR are for…of loops", async () => {
  const dir = fixture("mir-loops-in-turn");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;
pub struct D { pub line: u32, pub class_name: &'static str }
pub fn decorate(lines: &[D], inline: &[D]) -> u32 {
    let mut highlighted = HashMap::new();
    for decorator in lines {
        highlighted.insert(decorator.line, decorator.class_name);
    }
    let mut starts = HashMap::new();
    for decorator in inline {
        if starts.contains_key(&decorator.line) {
            panic!("already at {}", decorator.line);
        }
        starts.insert(decorator.line, decorator.class_name);
    }
    (highlighted.len() + starts.len()) as u32
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function decorate(lines, inline) {
  const highlighted = new Map();
  for (const decorator of lines) {
    highlighted.set(decorator.line, decorator.className);
  }
  const starts = new Map();
  for (const decorator of inline) {
    if (starts.has(decorator.line)) {
      throw new Error(\`already at \${decorator.line}\`);
    }
    starts.set(decorator.line, decorator.className);
  }
  return (highlighted.size + starts.size) >>> 0;
}`);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.decorate([{ line: 1, className: "a" }], [{ line: 2, className: "b" }])).toBe(2);
});

// From MIR (ADR 0364): a \`return\` the program wrote first, as written; a
// range's loop over an array's indices, whose length it only reads, a
// counted \`for\`; its index, checked by THIR's facts, read unchecked.
test("early returns and index loops from MIR read as written", async () => {
  const dir = fixture("mir-early-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn get_type(message: &str) -> &'static str {
    if message == "log" || message == "info" {
        return "info";
    }
    if message == "warn" {
        return "warning";
    }
    "error"
}
pub fn joined(args: &[String], mut formatted: String) -> String {
    if !args.is_empty() {
        for i in 0..args.len() {
            formatted += &format!(" {}", args[i]);
        }
    }
    formatted
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  if (message === "warn") {\n    return "warning";\n  }\n  return "error";\n');
  expect(js).toContain("    for (let i = 0; i < args.length; i++) {\n      formatted += ` ${args[i]}`;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.getType("warn"), lib.getType("x"), lib.joined(["a", "b"], "f")]).toEqual(["warning", "error", "f a b"]);
});

// ADR 0364: `for` over what `Object.keys` gives, a `Vec`, steps through
// its array: `Object.keys(o)` is no JS iterator.
test("a for over Object.keys steps through its array", async () => {
  const dir = fixture("object-keys-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn listed(o: &js::Unknown) -> u32 {
    let mut n = 0;
    for key in js::object::keys(o) {
        if !key.is_empty() {
            n += 1;
        }
    }
    n
}
pub fn first(o: &js::Unknown) -> Option<String> {
    let mut keys = js::object::keys(o).into_iter();
    keys.next()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.listed({ a: 1, b: 2 }), lib.listed({})]).toEqual([2, 0]);
  expect([lib.first({ a: 1, b: 2 }), lib.first({})]).toEqual(["a", undefined]);
});

// ADR 0313: what react.dev's Console writes, as a person writes it: `!x` of
// text that's `None` or empty; `i < args.length` of a range of a length
// nothing in the loop changes, and a deque's `args[i]` in it; `[...a, ...b]`
// of two chained; and no trailing `undefined` argument.
test("Console's loops, spreads and calls read as written", async () => {
  const dir = fixture("console-forms");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::VecDeque;

pub fn unescaped(escaped: Option<&str>) -> bool {
    escaped.is_none_or(str::is_empty)
}

pub fn joined(args: VecDeque<u32>) -> String {
    let mut formatted = String::new();
    for i in 0..args.len() {
        formatted += &format!(" {}", args[i]);
    }
    formatted
}

pub fn shifted(mut args: VecDeque<u32>) -> String {
    let mut take = || args.pop_front();
    take();
    let mut formatted = String::new();
    for i in 0..args.len() {
        formatted += &format!(" {}", args[i]);
    }
    formatted
}

pub fn drained(mut args: Vec<u32>) -> u32 {
    let mut total = 0;
    for _ in 0..args.len() {
        total += args.pop().unwrap_or(0);
    }
    total
}

pub fn both(prev: &VecDeque<u32>, next: Vec<u32>) -> VecDeque<u32> {
    prev.iter().copied().chain(next).collect()
}

fn greet(name: &str, title: Option<&str>) -> String {
    format!("{}{name}", title.unwrap_or(""))
}

pub fn hi() -> String {
    greet("Ada", None)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return !escaped;");
  expect(js).toContain("for (let i = 0; i < args.length; i++) {\n    formatted += ` ${args[i]}`;");
  // Changed before the loop only, by a closure even, as Rust's borrows
  // allow nothing else while the loop reads it.
  expect(js).toMatch(/export function shifted\(args\) \{[^]*for \(let i = 0; i < args\.length; i\+\+\) \{\n    formatted \+= ` \$\{args\[i\]\}`;/);
  // One whose body changes the length reads its end once, as Rust does.
  expect(js).toContain("const end = args.length;");
  expect(js).toContain("return [...prev, ...next];");
  expect(js).toContain('return greet("Ada");');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.unescaped(undefined), lib.unescaped(""), lib.unescaped("%")]).toEqual([true, true, false]);
  expect([lib.joined([1, 2]), lib.shifted([1, 2, 3]), lib.drained([1, 2, 3]), lib.both([1], [2, 3]), lib.hi()]).toEqual([" 1 2", " 2 3", 6, [1, 2, 3], "Ada"]);
});

// A `?` and a `return` in a loop, and a `while` in it: each `return` where
// it's written, and the `while` tested at its head, not MIR's one exit, a
// labeled block, nor a `while (true)`.
test("returns in a loop are where they're written", async () => {
  const dir = fixture("loop-returns");
  writeFileSync(join(dir, "lib.rs"), `pub fn numbers(src: &str) -> Result<Vec<f64>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(text.parse::<f64>().map_err(|_| format!("bad {text}"))?);
            continue;
        }
        return Err(format!("unexpected {c}"));
    }
    Ok(out)
}

pub fn tokens(src: &str) -> Result<Vec<u32>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '_' {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(text.parse::<u32>().map_err(|_| format!("bad {text}"))?);
            continue;
        }
        match c {
            '+' => out.push(0),
            _ => return Err(format!("unexpected {c}")),
        }
        i += 1;
    }
    Ok(out)
}

pub fn scan(v: &[u32]) -> u32 {
    let mut i = 0;
    let mut again = true;
    for &x in v {
        'inner: loop {
            if i < x {
                i += 1;
                continue;
            }
            for _ in 0..2 {
                if again {
                    again = false;
                    i = 0;
                    continue 'inner;
                }
            }
            return i * 10 + x;
        }
    }
    0
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  const numbers = js.slice(0, js.indexOf("function tokens"));
  // What the program wrote first, first.
  const tokens = js.slice(js.indexOf("function tokens"), js.indexOf("function scan"));
  expect(tokens).toContain('if (/^[0-9]$/.test(c) || c === "_") {\n      const start = i;');
  expect(numbers).not.toMatch(/\bbb\d+\b/);
  expect(numbers).not.toContain("while (true)");
  // In the program's order, its `?` a guard.
  expect(numbers).toContain('if (/^[0-9]$/.test(c)) {\n      const start = i;');
  expect(numbers).toContain('      if (result$1.TAG === "Err") {\n        return result$1;\n      }\n      out.push(result$1._0);\n');
  expect(js).toContain("while (i < chars.length && /^[0-9]$/.test(chars[i])) {");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.numbers("12 3"), lib.numbers("1x")]).toEqual([{ TAG: "Ok", _0: [12, 3] }, { TAG: "Err", _0: "unexpected x" }]);
  expect([lib.tokens("12+3"), lib.tokens("1-")]).toEqual([{ TAG: "Ok", _0: [12, 0, 3] }, { TAG: "Err", _0: "unexpected -" }]);
  // A loop whose rest jumps to it, from a loop of its own, stays one, by
  // the label the program gave it.
  expect(js).toContain("inner: while (true) {");
  expect([lib.scan([3]), lib.scan([])]).toEqual([33, 0]);
});

// A range's count the body doesn't read, `for _ in 0..n`: `i`.
test("an unread count of a range is i", async () => {
  const dir = fixture("unread-count");
  writeFileSync(join(dir, "lib.rs"), `pub fn repeat(s: &str, times: u32) -> String {
    let mut out = String::new();
    for _ in 0..times {
        out.push_str(s);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("for (let i = 0; i < times; i++) {");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.repeat("ab", 3), lib.repeat("x", 0)]).toEqual(["ababab", ""]);
});

// A function's tail `loop` that `break a` gives its value: `return a` where
// it breaks.
test("a tail loop's break with a value returns it", async () => {
  const dir = fixture("loop-break-value");
  writeFileSync(join(dir, "lib.rs"), `pub fn fib_loop(n: u32) -> u32 {
    let mut a = 0;
    let mut b = 1;
    let mut i = 0;
    loop {
        if i == n {
            break a;
        }
        let next = a + b;
        a = b;
        b = next;
        i += 1;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("    if (i === n) {\n      return a;\n    }");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.fibLoop(0), lib.fibLoop(10)]).toEqual([0, 55]);
});

test("a value returned after a loop, the function returning early before it, is returned once", async () => {
  const dir = fixture("mir-returned-after-loop");
  writeFileSync(join(dir, "lib.rs"), `pub fn parse(search: &str) -> Vec<String> {
    if search.is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    let queries: Vec<&str> = search.split('&').collect();
    for i in 0..queries.len() {
        let query = queries[i];
        if query.starts_with("a") {
            args.push(query.to_string());
        }
    }
    args
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`export function parse(search) {
  if (!search) {
    return [];
  }
  const args = [];
  const queries = search.split("&");
  for (let i = 0; i < queries.length; i++) {
    const query = queries[i];
    if (query.startsWith("a")) {
      args.push(query);
    }
  }
  return args;
}`);
});

test("a range loop from MIR whose end is a call holds it first, and a slice of `i..i + 1` reads its bounds", async () => {
  const dir = fixture("mir-range-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn plain(code: &str) -> u32 {
    let mut n = 0;
    for i in 0..code.len() {
        if i > 2 {
            n += 1;
        }
    }
    n
}
pub fn sliced(code: &str) -> u32 {
    let mut n = 0;
    for i in 0..3 {
        if &code[i..i + 1] == "a" {
            n += 1;
        }
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  const end = $byteLen(code);\n  for (let i = 0; i < end; i++) {\n");
  expect(js).toContain('    if ($strSlice(code, i, i + 1) === "a") {\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.plain("abcdef"), lib.sliced("aba")]).toEqual([3, 2]);
});

test("a `while` loop from MIR tests where it's written", async () => {
  const dir = fixture("mir-while-tests");
  writeFileSync(join(dir, "lib.rs"), `pub fn trimmed(mut messages: Vec<u32>, max: usize) -> Vec<u32> {
    while messages.len() > max {
        messages.remove(0);
    }
    messages
}
pub fn counted(mut n: u32) -> u32 {
    let mut steps = 0;
    while n > 1 {
        n /= 2;
        steps += 1;
    }
    steps
}
pub fn stopped(mut n: u32) -> u32 {
    while n > 1 {
        if n == 7 {
            return 70;
        }
        if n == 9 {
            break;
        }
        n /= 2;
    }
    n + 100
}
pub fn looped(mut n: u32) -> u32 {
    loop {
        if n <= 1 {
            return 1;
        }
        if n == 9 {
            break;
        }
        n /= 2;
    }
    n + 100
}
pub fn halved(mut n: u32, out: &mut Vec<u32>) {
    while n > 1 {
        n /= 2;
        if n == 7 {
            break;
        }
        out.push(n);
    }
    out.push(0);
}
pub fn spun(mut n: u32) -> u32 {
    loop {
        if n <= 1 {
            return n;
        }
        n /= 2;
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  while (messages.length > max) {\n    $removeAt(messages, 0);\n  }\n  return messages;\n");
  expect(js).toContain("  while (n > 1) {\n    n = (n / 2) >>> 0;\n    steps = (steps + 1) >>> 0;\n  }\n  return steps;\n");
  expect(js).toContain("  while (n > 1) {\n");
  // A `loop` the program wrote stays one.
  expect(js).toContain("export function looped(n) {\n  while (true) {\n    if (n <= 1) {\n      return 1;\n    }\n");
  expect(js).toContain("export function spun(n) {\n  while (true) {\n    if (n <= 1) {\n      return n;\n    }\n");
  expect(js).toContain("  while (n > 1) {\n    if (n === 7) {\n      return 70;\n    }\n    if (n === 9) {\n      break;\n    }\n    n = (n / 2) >>> 0;\n  }\n  return n + 100;\n");
  const lib = await import(join(dir, "mir.js"));
  const pushed: number[] = [];
  lib.halved(30, pushed);
  expect([lib.trimmed([1, 2, 3], 1), lib.counted(9), pushed, lib.stopped(15), lib.stopped(9), lib.stopped(40), lib.looped(18), lib.looped(4)]).toEqual([[3], 3, [15, 0], 70, 109, 101, 109, 1]);
});

test("a loop's `continue`s from MIR are where the program wrote them", async () => {
  const dir = fixture("mir-written-continues");
  writeFileSync(join(dir, "lib.rs"), `pub struct Route { pub path: Option<String>, pub header: bool, pub title: Option<String> }
pub fn pages(routes: &[Route], out: &mut Vec<String>) {
    for route in routes {
        if route.header {
            continue;
        }
        if let Some(p) = &route.path && p.starts_with("http") {
            continue;
        }
        if let Some(t) = &route.title {
            out.push(t.clone());
        } else if let Some(p) = &route.path {
            out.push(p.clone());
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`  for (const route of routes) {
    if (route.header) {
      continue;
    }
    if (route.path != null && route.path.startsWith("http")) {
      continue;
    }
    if (route.title != null) {
      out.push(route.title);
    } else if (route.path != null) {
      out.push(route.path);
    }
  }`);
});

// A loop the program leaves with \`break\`, its function's value after it,
// from MIR, breaks, the value returned after it, as written:
// not the function's \`return\` moved into the loop.
test("a loop's break from MIR is a break, the return after it", async () => {
  const dir = fixture("mir-loop-break-return");
  writeFileSync(join(dir, "lib.rs"), `pub fn odds() -> Vec<i32> {
    let mut out = Vec::new();
    let mut i = 0;
    loop {
        i += 1;
        if i > 7 {
            break;
        }
        out.push(i);
    }
    out
}
pub fn drained(mut stack: Vec<u32>) -> String {
    let mut out = String::new();
    while let Some(top) = stack.pop() {
        out.push_str(&top.to_string());
    }
    out
}
pub fn early(xs: &[i32]) -> i32 {
    for &x in xs {
        if x > 2 {
            return x;
        }
    }
    0
}
pub fn root(n: u32) -> u32 {
    let mut i = 0;
    loop {
        i += 1;
        if i * i > n {
            return i;
        }
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    if (i > 7) {\n      break;\n    }\n");
  expect(js).toContain("  }\n  return out;\n}\nexport function drained");
  expect(js).toMatch(/\n  }\n  return out;\n}\nexport function early/);
  // A \`return\` the program wrote stays.
  expect(js).toContain("      return x;\n");
  expect(js).toContain("      return i;\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.odds(), lib.drained([1, 2]), lib.early([1, 5]), lib.early([]), lib.root(10)]).toEqual([[1, 2, 3, 4, 5, 6, 7], "21", 5, 0, 4]);
});

// An \`else if\` the program wrote at a loop's end, after an \`if\` it ended
// with a \`continue\`, stays an \`else if\` from MIR: not a \`continue\` of
// its own and an \`if\` after, as react.dev's llms.txt sections its routes.
test("an else-if at a loop's end from MIR stays one past a written continue", async () => {
  const dir = fixture("mir-else-if-after-continue");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
struct RouteItem {
    title: Option<String>,
    path: Option<String>,
    routes: Option<Vec<RouteItem>>,
    has_section_header: Option<bool>,
    section_header: Option<String>,
}

struct Sidebar {
    title: String,
    routes: Vec<RouteItem>,
}

struct Page {
    title: String,
    url: String,
}

struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

struct Section {
    heading: Option<String>,
    pages: Vec<Page>,
    sub_groups: Vec<SubGroup>,
}

// Clean up section header names (remove version placeholders)
fn cleanSectionHeader(header: &str) -> String { header.trim().to_string() }
pub fn extractSectionedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut currentSection: Option<Section> = None;

    for route in routes {
        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Start a new section when we hit a section header
        if route.has_section_header == Some(true)
            && let Some(sectionHeader) = &route.section_header
        {
            if let Some(section) = currentSection {
                sections.push(section);
            }
            currentSection = Some(Section {
                heading: Some(cleanSectionHeader(sectionHeader)),
                pages: Vec::new(),
                sub_groups: Vec::new(),
            });
            continue;
        }

        // If no section started yet, skip
        let Some(currentSection) = &mut currentSection else {
            continue;
        };

        // Route with children - create a sub-group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut subGroup = SubGroup {
                heading: title.clone(),
                pages: Vec::new(),
            };

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                subGroup.pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    subGroup.pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !subGroup.pages.is_empty() {
                currentSection.sub_groups.push(subGroup);
            }
        }
        // Single page without children
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            currentSection.pages.push(Page {
                title: title.clone(),
                url: format!("{baseUrl}{path}.md"),
            });
        }
    }

    // Don't forget the last section
    if let Some(section) = currentSection {
        sections.push(section);
    }

    sections
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // Its comment before the `else`, as upstream has it (ADR 0444, amended).
  expect(js).toContain("      }\n    }\n    // Single page without children\n    else if (route.title != null && route.path != null) {\n");
  // Nor a \`continue\` where the body ends anyway.
  expect(js).not.toMatch(/\n      continue;\n    } else if/);
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, sectionHeader: "A" },
    { title: "t", path: "/p" },
    { title: "g", routes: [{ title: "c", path: "/c" }] },
  ];
  expect(lib.extractSectionedRoutes(routes, "x").map((s: { heading: string; pages: unknown[]; subGroups: unknown[] }) => [s.heading, s.pages.length, s.subGroups.length])).toEqual([["A", 1, 1]]);
});

// react.dev's llms.txt groups its routes: an `else if` the loop's body ends
// with, after an `if` that doesn't end in a `continue`, stays an `else if`,
// not a `continue` MIR's block for it makes and an `if` after.
test("react.dev's llms.txt groups its routes with an else-if at the loop's end", async () => {
  const dir = fixture("mir-grouped-routes");
  writeFileSync(join(dir, "lib.rs"), `#![allow(non_snake_case, dead_code)]
pub struct RouteItem {
    title: Option<String>,
    path: Option<String>,
    routes: Option<Vec<RouteItem>>,
    has_section_header: Option<bool>,
}

pub struct Page {
    title: String,
    url: String,
}

pub struct SubGroup {
    heading: String,
    pages: Vec<Page>,
}

pub fn extractGroupedRoutes(routes: &[RouteItem], baseUrl: &str) -> Vec<SubGroup> {
    let mut groups: Vec<SubGroup> = Vec::new();

    for route in routes {
        // Skip section headers
        if route.has_section_header == Some(true) {
            continue;
        }

        // Skip external links
        if route.path.as_ref().is_some_and(|path| path.starts_with("http")) {
            continue;
        }

        // Route with children - create a group
        if let Some(title) = &route.title
            && let Some(children) = &route.routes
            && !children.is_empty()
        {
            let mut pages: Vec<Page> = Vec::new();

            // Include parent page if it has a path
            if let Some(path) = &route.path {
                pages.push(Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                });
            }

            // Add child pages
            for child in children {
                if let Some(childTitle) = &child.title
                    && let Some(childPath) = &child.path
                    && !childPath.starts_with("http")
                {
                    pages.push(Page {
                        title: childTitle.clone(),
                        url: format!("{baseUrl}{childPath}.md"),
                    });
                }
            }

            if !pages.is_empty() {
                groups.push(SubGroup {
                    heading: title.clone(),
                    pages,
                });
            }
        }
        // Single page without children - group under its own heading
        else if let Some(title) = &route.title
            && let Some(path) = &route.path
        {
            groups.push(SubGroup {
                heading: title.clone(),
                pages: vec![Page {
                    title: title.clone(),
                    url: format!("{baseUrl}{path}.md"),
                }],
            });
        }
    }

    groups
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("        groups.push({\n          heading: route.title,\n          pages,\n        });\n      }\n    }\n    // Single page without children - group under its own heading\n    else if (route.title != null && route.path != null) {\n");
  expect(js).not.toContain("\n    if (route.title != null && route.path != null) {");
  const lib = await import(join(dir, "mir.js"));
  const routes = [
    { hasSectionHeader: true, title: "H", path: "/h" },
    { title: "E", path: "https://e" },
    { title: "G", path: "/g", routes: [{ title: "c", path: "/c" }, { title: "x", path: "http://x" }] },
    { title: "T", path: "/t" },
    { title: "N" },
  ];
  const grouped = lib.extractGroupedRoutes(routes, "b").map((g: { heading: string; pages: { url: string }[] }) => [g.heading, g.pages.map((p) => p.url)]);
  expect(grouped).toEqual([["G", ["b/g.md", "b/c.md"]], ["T", ["b/t.md"]]]);
});

// react.dev's CodeBlock places its inline decorators: a loop's body ending
// in an `if`'s branch, here its last insert past the `panic!`s' `if`s, ends
// with no `continue` MIR's block for it makes, which does nothing there.
test("react.dev's CodeBlock places its decorators with no continue at the loop's end", async () => {
  const dir = fixture("mir-decorator-ends");
  writeFileSync(join(dir, "lib.rs"), `use std::collections::HashMap;

pub struct InlineDecorator {
    pub line: u32,
    pub start_column: u32,
    pub end_column: u32,
    pub class_name: String,
}

pub fn ends(lines: &[&str], inline_decorators: Vec<InlineDecorator>) -> Vec<(u32, String)> {
    let mut decorator_starts = HashMap::new();
    let mut decorator_ends = HashMap::new();
    for decorator in inline_decorators {
        // Find where inline highlight starts and ends.
        let mut decorator_start = 0;
        for i in 0..decorator.line - 1 {
            decorator_start += lines[i as usize].len() as u32 + 1;
        }
        decorator_start += decorator.start_column;
        let decorator_end = decorator_start + (decorator.end_column - decorator.start_column);
        if decorator_starts.contains_key(&decorator_start) {
            panic!("Already opened decorator at {decorator_start}");
        }
        decorator_starts.insert(decorator_start, decorator.class_name.clone());
        if decorator_ends.contains_key(&decorator_end) {
            panic!("Already closed decorator at {decorator_end}");
        }
        decorator_ends.insert(decorator_end, decorator.class_name);
    }
    let mut out: Vec<(u32, String)> = decorator_ends.into_iter().collect();
    out.sort();
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    decoratorEnds.set(decoratorEnd, decorator.className);\n  }\n");
  expect(js).not.toContain("continue;");
  const lib = await import(join(dir, "mir.js"));
  const decorator = (line: number, startColumn: number, endColumn: number, className: string) => ({ line, startColumn, endColumn, className });
  expect(lib.ends(["ab", "cde"], [decorator(2, 1, 3, "k"), decorator(1, 0, 1, "s")])).toEqual([[1, "s"], [6, "k"]]);
  expect(() => lib.ends(["ab"], [decorator(1, 0, 1, "a"), decorator(1, 1, 1, "b")])).toThrow("Already closed decorator at 1");
});

// A slice's items with their indices, `iter().enumerate()`, from MIR are
// the array's `entries()`, and a loop's label the program's own; a
// `break 'outer` out of the loop inside keeps both loops.
test("nested loops from MIR read as written: entries, labels, and a labeled break", async () => {
  const dir = fixture("mir-nested-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn place(grid: &[Vec<u32>]) -> Option<(usize, usize)> {
    for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v == 4 {
                return Some((r, c));
            }
        }
    }
    None
}
pub fn skip(grid: &[Vec<u32>]) -> u32 {
    let mut n = 0;
    'rows: for row in grid {
        for &v in row {
            if v == 0 {
                continue 'rows;
            }
            n += v;
        }
    }
    n
}
pub fn first(grid: &[Vec<u32>]) -> Option<u32> {
    let mut found = None;
    'outer: for row in grid {
        for &v in row {
            if v > 3 {
                found = Some(v);
                break 'outer;
            }
        }
    }
    found
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  for (const [r, row] of grid.entries()) {\n    for (const [c, v] of row.entries()) {\n");
  expect(js).toContain("  rows: for (const row of grid) {");
  expect(js).toContain("        continue rows;");
  expect(js).toContain(`  outer: for (const row of grid) {
    for (const v of row) {
      if (v > 3) {
        found = v;
        break outer;
      }
    }
  }
  return found;`);
  const lib = await import(join(dir, "mir.js"));
  const grid = [[1, 2], [0, 5], [3, 4]];
  expect([lib.place(grid), lib.place([[1]]), lib.skip(grid), lib.first(grid), lib.first([[1]])]).toEqual([[2, 1], undefined, 10, 5, undefined]);
});

// A `while` in a closure from MIR, whose test reads what MIR held first,
// `const _7 = messages.length;`, is the `while` written, as react.dev's
// Console keeps its latest messages: not `while (true)` and a `return`;
// incremental too, where the closure's spans are its own.
test("a while in a closure from MIR keeps its test", async () => {
  const dir = fixture("closure-while");
  writeFileSync(join(dir, "lib.rs"), `pub fn listen(on: &dyn Fn(u32) -> Box<dyn Fn(Vec<u32>) -> Vec<u32>>) -> Vec<u32> {
    let f = on(2);
    f(vec![1, 2, 3, 4])
}
pub fn run() -> Vec<u32> {
    listen(&|keep| {
        Box::new(move |prev: Vec<u32>| {
            let mut messages = prev;
            while messages.len() > keep as usize {
                messages.remove(0);
            }
            messages
        })
    })
}
`);
  // Incremental, as an app's build is: spans of a closure's body hold it.
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "-C", `incremental=${join(dir, "incremental")}`]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    while (messages.length > keep) {\n      $removeAt(messages, 0);\n    }\n    return messages;\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.run()).toEqual([3, 4]);
});

// A `while` that ends an `if`'s branch, with code after the `if`, whose
// exit left MIR's block, is its test, as ReactFiberLane's
// `getEntangledLanes` walks its lanes (ADR 0702, amended).
test("a while that ends an if's branch keeps its test", async () => {
  const dir = fixture("while-ending-branch");
  writeFileSync(join(dir, "lib.rs"), `pub fn entangled(lanes: i32, all: i32) -> i32 {
    let mut entangled = lanes;
    if all != 0 {
        let mut rest = lanes & all;
        while rest > 0 {
            entangled |= rest << 1;
            rest &= rest - 1;
        }
    }
    entangled
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    while (rest > 0) {\n");
  expect(js).not.toContain("while (true)");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.entangled(0b101, 0b100), lib.entangled(0b101, 0)]).toEqual([0b1101, 0b101]);
});

// A loop's `if`s the program ends with `continue`,
// `if c == "{" { depth += 1; continue; }`, are written so, each then what
// follows, not an `else if` chain, as react.dev's createFileMap splits a
// meta string.
test("continues a loop's ifs end with are kept", async () => {
  const dir = fixture("ending-continues");
  writeFileSync(join(dir, "lib.rs"), `pub fn split(meta: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    for c in meta.chars() {
        let c = c.to_string();
        if c == "{" {
            depth += 1;
            continue;
        }
        if c == "}" {
            depth -= 1;
            continue;
        }
        if depth > 0 {
            continue;
        }
        if c == " " {
            if !current.is_empty() {
                tokens.push(current);
                current = String::new();
            }
            continue;
        }
        current += &c;
    }
    tokens
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('      depth = (depth + 1) | 0;\n      continue;\n    }\n    if (c === "}") {');
  expect(js).toContain("    if (depth > 0) {\n      continue;\n    }");
  expect(js).not.toContain("else");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.split("a {b c} d e")).toEqual(["a", "d"]);
});

// A `let … else { continue; }` the program wrote, as react.dev's llmsTxt
// has of its current section, stays the guard it is from MIR, `if (!s) {
// continue; }` and what follows, whatever its test.
test("a let-else's written continue from MIR stays a guard", async () => {
  const dir = fixture("let-else-continue");
  writeFileSync(join(dir, "lib.rs"), `pub struct Section { pub pages: Vec<u32> }
pub fn sections(items: &[u32]) -> Vec<Section> {
    let mut out = Vec::new();
    let mut current: Option<Section> = None;
    for &item in items {
        if item == 0 {
            if let Some(section) = current {
                out.push(section);
            }
            current = Some(Section { pages: Vec::new() });
            continue;
        }
        let Some(current) = &mut current else {
            continue;
        };
        if item > 10 {
            current.pages.push(item / 10);
        } else {
            current.pages.push(item);
        }
    }
    if let Some(section) = current {
        out.push(section);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain("    if (!current) {\n      continue;\n    }\n    if (item > 10) {\n");
  expect((await import(join(dir, "mir.js"))).sections([5, 0, 30, 2, 0, 7])).toEqual([{ pages: [3, 2] }, { pages: [7] }]);
});

// An arm that ends in a loop, which leaves the match where it ends:
// the loop's own `break`, not a labeled block's.
test("a match arm that ends in a loop leaves it with the loop's break", async () => {
  const dir = fixture("arm-loop-exit");
  writeFileSync(join(dir, "lib.rs"), `pub fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            'a'..='z' => {
                let mut w = String::new();
                while let Some(c) = chars.next_if(|c| c.is_ascii_lowercase()) {
                    w.push(c);
                }
                out.push(w);
            }
            _ => {
                chars.next();
            }
        }
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toMatch(/\bbb\d+\b/);
  expect(js).toContain(`    if (c >= "a" && c <= "z") {\n      let w = "";`);
  expect(js).toContain(`          out.push(w);\n          break;\n`);
  const lib = await import(join(dir, "lib.js"));
  expect(lib.words("ab cd,e")).toEqual(["ab", "cd", "e"]);
  expect(lib.words("")).toEqual([]);
});

// Counted loops in each branch of a loop's body, after another loop: each
// its own `for`, the branches' end where the `if` does, not the outer loop's.
test("counted loops in a loop's branches stay where they're written", async () => {
  const dir = fixture("branch-loops");
  writeFileSync(join(dir, "lib.rs"), `pub fn scaled(seeds: &[u32]) -> Vec<f32> {
    let mut out = Vec::new();
    let mut power = 1.0f32;
    for _ in 0..3 {
        power *= 2.0;
    }
    out.push(power);
    for &seed in seeds {
        let mut x = seed as f32;
        let scale = seed % 7;
        if scale < 3 {
            for _ in 0..scale + 1 {
                x *= 0.5;
            }
        } else {
            for _ in 0..scale - 3 {
                x *= 2.0;
            }
        }
        if seed % 2 == 0 {
            x = -x;
        }
        out.push(x);
    }
    out
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("$rangeNext");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.scaled([1, 5, 10, 13])).toEqual([8, 0.25, 20, -10, 104]);
});

// A `for` whose binding is `mut`, which MIR declares before the loop and
// assigns each turn: a `for…of` all the same, of an iterator stepped
// first too.
test("a for loop of a mut binding is a for...of", async () => {
  const dir = fixture("for-mut-binding");
  writeFileSync(join(dir, "lib.rs"), `pub fn plain_mut(v: Vec<Vec<u32>>) -> usize {
    let mut n = 0;
    for mut row in v {
        row.push(0);
        n += row.len();
    }
    n
}
pub fn rest_mut(v: Vec<Vec<u32>>) -> usize {
    let mut it = v.into_iter();
    it.next();
    let mut n = 0;
    for mut row in it {
        row.push(0);
        n += row.len();
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  for (const row of v) {\n    row.push(0);");
  expect(js).toContain("  for (const row of it) {\n    row.push(0);");
  expect(js).not.toContain("let row;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plainMut([[1], [2, 3]]), lib.restMut([[1], [2, 3], [4]])]).toEqual([5, 5]);
});

// React's describeObjectForErrorMessage declares `let substr;` in each
// loop's body, where its `if` chain sets it: a `let` the program declares
// in a loop's body, set on each way through the block MIR makes of the
// chain before it's read, is in that body, not hoisted out of the loop.
test("a let a loop's body declares, set in a chain's block, is in the body", async () => {
  const dir = fixture("loop-body-lets");
  writeFileSync(join(dir, "lib.rs"), `use js::{Kind, Unknown};

pub fn joined(items: &[Option<&Unknown>], expanded: Option<&str>) -> (String, Option<u32>) {
    let mut str = "[".to_string();
    let mut start = None;
    for (i, &value) in items.iter().enumerate() {
        if i > 0 {
            str += ", ";
        }
        let substr = if let Some(object) = value
            && let Kind::Object(_) | Kind::Array(_) = js::classify(Some(object))
        {
            "{...}".to_string()
        } else {
            js::string(value)
        };
        if expanded == Some(i.to_string().as_str()) {
            start = Some(js::string::length(&str));
        }
        str += &substr;
    }
    str += "]";
    (str, start)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('    if (i > 0) {\n      str += ", ";\n    }\n    let substr;\n    if (value != null && typeof value === "object") {\n');
  expect(js).not.toContain("  let substr;\n  for");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.joined([1, { a: 1 }, "x", undefined], "2")).toEqual(["[1, {...}, x, undefined]", 11]);
});
