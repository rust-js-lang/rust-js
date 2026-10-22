// What the compiler writes of functions and closures: a callback that's the
// function it calls, a function declared in a body, what a closure copies,
// generics' copies, and temporaries read in place. Each test is a program of
// its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

// ADR 0610: a function that only calls a module variable's function with
// its parameters is that variable, by the function's name, as React's
// scheduler has \`let getCurrentTime; if (..) { getCurrentTime = () => .. }\`.
// Not where the module's load calls it before the variable is set.
test("a function that only calls a module variable's function is the variable", async () => {
  const dir = fixture("forwarding-functions");
  writeFileSync(join(dir, "lib.rs"), `thread_local! {
    static GET_TIME: Box<dyn Fn(f64) -> f64> = if js::math::random() < 2.0 {
        Box::new(|n| n + 1.0)
    } else {
        let k = js::math::random();
        Box::new(move |n| n * k)
    };
    static SCALE: Box<dyn Fn(f64) -> f64> = if js::math::random() < 2.0 {
        Box::new(|n| n * 2.0)
    } else {
        Box::new(|n| n)
    };
}

pub fn scale(n: f64) -> f64 {
    SCALE.with(|scale| scale(n))
}

thread_local! {
    static EARLY: Box<dyn Fn(f64) -> f64> = if js::math::random() < 2.0 {
        Box::new(|n| n + 3.0)
    } else {
        let k = js::math::random();
        Box::new(move |n| n * k)
    };
    static FIRST: f64 = early(1.0);
}

// Called where the module loads: a function, which its hoisting lets run.
pub fn early(n: f64) -> f64 {
    EARLY.with(|early| early(n))
}

pub fn first() -> f64 {
    FIRST.with(|first| *first)
}

pub fn get_time(n: f64) -> f64 {
    GET_TIME.with(|get_time| get_time(n))
}

pub fn later() -> f64 {
    get_time(1.0)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("let getTime;\nif (Math.random() < 2) {\n  getTime = (n) => n + 1;\n");
  expect(js).not.toContain("function getTime");
  expect(js).toContain("export { getTime };");
  expect(js).toContain("export const scale = Math.random() < 2 ? (n) => n * 2 : (n) => n;");
  const lib = await import(join(dir, "lib.js"));
  expect(js).toContain("export function early(n) {");
  expect([lib.getTime(1), lib.later(), lib.scale(2), lib.first()]).toEqual([2, 2, 4, 4]);
});

// ADR 0607: a variable's field is given what a call gives in place,
// \`more.has = work();\`, as React's scheduler has \`hasMoreWork =
// flushWork(currentTime);\`: what makes the value can't make the variable
// another, a closure's \`more = ..\` assigning the object in place.
test("a field is given a call's value in place", async () => {
  const dir = fixture("field-given-in-place");
  writeFileSync(join(dir, "lib.rs"), `pub struct More {
    pub has: bool,
}

fn work() -> bool {
    false
}

pub fn plain() -> bool {
    let mut more = More { has: true };
    more.has = work();
    more.has
}

pub fn rebound() -> bool {
    let mut more = More { has: false };
    let mut replace = || {
        more = More { has: true };
        false
    };
    more.has = replace();
    more.has
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  more.has = work();\n");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.plain(), lib.rebound()]).toEqual([false, false]);
});

// ADR 0279: a callback that only passes what it's given on, `(x) => double(x)`,
// is the function itself, `xs.map(double)`, as react.dev's
// toCommaSeparatedList has `array.map(renderCallback)`, where it's Rust's, which
// takes no more than it's given: a JS function's, `parseFloat`, may read
// the index too.
test("a callback that only passes its arguments on is the function", async () => {
  const dir = fixture("eta");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "parseFloat")] #[allow(unused_variables)] fn parse_float(text: &str) -> f64 { unreachable!() }

fn double(x: &u32) -> u32 {
    x * 2
}

pub fn doubled(xs: &[u32]) -> Vec<u32> {
    xs.iter().map(|x| double(x)).collect()
}

pub fn rendered(xs: &[u32], f: impl Fn(&u32, usize) -> u32) -> Vec<u32> {
    xs.iter().enumerate().map(|(i, x)| f(x, i)).collect()
}

pub fn parsed(xs: &[String]) -> Vec<f64> {
    xs.iter().map(|x| parse_float(x)).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return xs.map(double);");
  expect(js).toContain("return xs.map(f);");
  expect(js).toContain("return xs.map((x) => parseFloat(x));");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.doubled([1, 2]), lib.rendered([5, 6], (x: number, i: number) => x + i), lib.parsed(["1.5", "2"])]).toEqual([[2, 4], [5, 7], [1.5, 2]]);
});

// ADR 0029: an awaited reference given to a generic `&T` is awaited, though
// rustc reborrows it inside the `.await`.
test("an awaited reference given to a generic function is awaited", async () => {
  const dir = fixture("await-generic");
  writeFileSync(join(dir, "lib.rs"), `use js::Promise;

#[cfg_attr(rust_js, rust_js::link_name = "node:timers/promises#setTimeout")]
#[allow(unused_variables)]
fn later(ms: u32, value: &str) -> Promise<&'static str> {
    unreachable!()
}

fn same<T: ?Sized>(x: &T) -> &T {
    x
}

pub async fn passed(text: &str) -> String {
    same(later(0, text).await).to_string()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "lib.js"), "utf8")).toContain("return same(await setTimeout(0, text));");
  expect(await (await import(join(dir, "lib.js"))).passed("text")).toBe("text");
});

// A temporary MIR (ADR 0364) makes of a variable nothing changes, or of a
// constant, is what it holds where it's read: of `assert_eq!`'s pair, each
// value compared. Its check, a branch that panics, comes first, as a person
// writes it.
test("a temporary from MIR of what reads the same anywhere is what it holds", async () => {
  const dir = fixture("mir-aliases");
  writeFileSync(join(dir, "lib.rs"), `use std::any::type_name;
pub fn describe<T>(_: &T) -> String {
    format!("{} {}", type_name::<T>(), size_of::<T>())
}
pub fn checked(n: u32) -> u32 {
    assert_eq!(n, 6, "not six");
    n
}
pub fn small(n: u32) -> u32 {
    if n > 9 {
        panic!("big");
    }
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return `${TName} ${TSize}`;\n");
  expect(js).toContain("  if (n !== 6) {\n    ");
  expect(js).toContain("  }\n  return n;\n");
  expect(js).toContain("  if (n > 9) {\n    ");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.checked(6)).toBe(6);
  expect(() => lib.checked(7)).toThrow();
  expect(lib.small(9)).toBe(9);
});

// ADR 0308: a function written in a body is declared there, where what
// uses it reads it.
test("a function written in a body is declared in it", async () => {
  const dir = fixture("local-functions");
  writeFileSync(join(dir, "lib.rs"), `pub fn doubled(n: u32) -> u32 {
    fn twice(n: u32) -> u32 {
        n * 2
    }
    let apply = |f: fn(u32) -> u32| f(n);
    if n > 10 {
        fn big() -> u32 {
            1000
        }
        return big();
    }
    apply(twice)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.doubled(3), lib.doubled(11)]).toEqual([6, 1000]);
});

// ADR 0020: only a mutated type is copied, and that's decided per type: a
// mutated `Pair<u32>` isn't a reason to copy a `Pair<bool>`. A generic
// function that mutates `Holder<T>` may mutate any `Holder<..>`, though.
test("copies are made for the mutated instantiations of a generic type only", async () => {
  const { fixture, compiler } = await import("./support");
  const { writeFileSync } = await import("node:fs");
  const dir = fixture("copies");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Pair<T: Copy> {
    pub a: T,
    pub b: T,
}

#[derive(Clone, Copy)]
pub struct Holder<T: Copy> {
    pub value: T,
}

pub fn bumped(p: Pair<u32>) -> (Pair<u32>, Pair<u32>) {
    let mut q = p;
    q.a += 1;
    (p, q)
}

pub fn twice(p: Pair<bool>) -> (Pair<bool>, Pair<bool>) {
    let q = p;
    (p, q)
}

pub fn set<T: Copy>(holder: &mut Holder<T>, value: T) {
    holder.value = value;
}

pub fn both(h: Holder<bool>) -> (Holder<bool>, Holder<bool>) {
    let mut g = h;
    set(&mut g, !h.value);
    (h, g)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = await Bun.file(join(dir, "lib.js")).text();
  expect(js).toContain("export function bumped(p) {\n  const q = { ...p };");
  expect(js).toContain("export function twice(p) {\n  const q = p;");
  expect(js).toContain("export function both(h) {\n  const g = { ...h };");
  const module = await import(join(dir, "lib.js"));
  expect(module.bumped({ a: 1, b: 2 })).toEqual([{ a: 1, b: 2 }, { a: 2, b: 2 }]);
  expect(module.twice({ a: true, b: false })).toEqual([{ a: true, b: false }, { a: true, b: false }]);
  expect(module.both({ value: true })).toEqual([{ value: true }, { value: false }]);
});

// A JS module's namespace, `#*.Root`, whose bindings a Rust module holds,
// is imported by that module's name, as react.dev's BrandMenu has
// `import * as ContextMenu` and `<ContextMenu.Root>`.
// A closure that only spawns an async block is an async arrow, as
// react.dev's BrandMenu writes `onSelect={async () => { await
// navigator.clipboard.writeText(..) }}`: it starts the same, and gives
// back a promise where nothing reads its `()` (ADR 0257). From MIR too
// (ADR 0364).
test("a closure that only spawns an async block is an async arrow", async () => {
  const dir = fixture("spawning-closure");
  writeFileSync(join(dir, "copy.js"), "export const copied = [];\nexport async function copy(text) { copied.push(text); }\n");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "./copy.js#copy")] #[allow(unused_variables)] fn copy(text: &str) -> js::Promise<()> { unreachable!() }

pub fn handler() -> Box<dyn Fn()> {
    Box::new(|| {
        js::spawn(Box::new(async {
            copy("#58C4DC").await;
        }))
    })
}
`);
  const { copied } = await import(join(dir, "copy.js"));
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return async () => {\n    await copy("#58C4DC");\n  };');
  const lib = await import(join(dir, "mir.js"));
  copied.length = 0;
  await lib.handler()();
  expect(copied).toEqual(["#58C4DC"]);
});

// A value shown is only read, through the reference `format_args!` takes,
// which nothing can change before it's shown: a `Copy` one changed elsewhere
// is shown in place, not copied first, as a read of it by value is (ADR 0020).
test("a formatted value is read in place, not copied", async () => {
  const dir = fixture("format-in-place");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Debug)]
pub struct P {
    pub x: i32,
}
pub fn show(mut p: P) -> String {
    let q = p;
    p.x += 1;
    let all = [q, p];
    format!("{q:?} {p:?} {all:?}")
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const q = { ...p };");
  expect(js).toContain("${pDebug_fmt(q)} ${pDebug_fmt(p)} [${all.map((item) => pDebug_fmt(item))");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.show({ x: 1 })).toBe("P { x: 1 } P { x: 2 } [P { x: 1 }, P { x: 2 }]");
});

// ADR 0289: a `T: Copy` takes a copy function only where it may be given
// a value a copy isn't, one changed in place: react.dev's Preview debounces
// an `Option<&SandpackError>`, whose copy is itself, so `useDebounced(value)`
// is called as the original does. One a caller outside the crate may call,
// or given a struct that changes, still takes one.
test("a Copy bound takes a copy only where a copy isn't the value", async () => {
  const dir = fixture("copy-bounds");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Point {
    pub x: u32,
}

fn twice<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

fn kept<T: Copy>(value: T) -> T {
    let copy = value;
    copy
}

pub fn named(name: Option<&'static str>) -> (Option<&'static str>, Option<&'static str>) {
    twice(name)
}

pub fn moved(p: Point) -> u32 {
    let mut q = kept(p);
    q.x += 1;
    p.x + q.x
}

fn inner<T: Copy>(value: T) -> (T, T) {
    (value, value)
}

fn relay<T: Copy>(value: T) -> (T, T) {
    inner(value)
}

pub fn relayed(p: Point) -> u32 {
    let mut pair = relay(p);
    pair.0.x += 1;
    pair.0.x + pair.1.x
}

pub struct Holder<T> {
    value: T,
}

impl<T: Copy> Holder<T> {
    fn get(&self) -> T {
        self.value
    }
}

pub fn held(p: Point) -> u32 {
    let h = Holder { value: p };
    let mut q = h.get();
    q.x += 1;
    h.value.x + q.x
}

// Called through a dictionary, a trait's method takes one whatever its
// direct callers give.
trait Dup {
    fn dup<T: Copy>(&self, value: T) -> (T, T);
}

struct Twin;

impl Dup for Twin {
    fn dup<T: Copy>(&self, value: T) -> (T, T) {
        (value, value)
    }
}

fn through<S: Dup>(s: &S) -> (&'static str, &'static str) {
    s.dup("a")
}

pub fn duped() -> (&'static str, &'static str) {
    through(&Twin)
}

// A trait impl's own bound, given with its dictionary, as rustc's
// traits/conditional-dispatch.rs has it.
trait Again {
    fn again(&self) -> Self;
}

impl<T: Copy> Again for Option<T> {
    fn again(&self) -> Self {
        *self
    }
}

trait Get {
    fn get(&self) -> Self;
}

impl<T: Again> Get for T {
    fn get(&self) -> T {
        self.again()
    }
}

fn get_it<T: Get>(t: &T) -> T {
    t.get()
}

pub fn got() -> Option<u32> {
    get_it(&Some(4))
}

pub fn shared<T: Copy>(value: T) -> T {
    value
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("function twice(value) {");
  expect(js).toContain("return twice(name);");
  expect(js).toContain("function kept(value, TCopy) {");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.named("a"), lib.moved({ x: 1 }), lib.relayed({ x: 1 }), lib.held({ x: 1 }), lib.duped(), lib.got()]).toEqual([["a", "a"], 3, 3, 3, ["a", "a"], 4]);
  // A library's public one may be given anything by its consumers.
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "library", "lib.js"), "--library", "--manifest", join(dir, "library", "lib.manifest.json"),
    "--", "--crate-name", "shared", `--emit=metadata=${join(dir, "library", "libshared.rmeta")}`]);
  expect(readFileSync(join(dir, "library", "lib.js"), "utf8")).toContain("export function shared(value, TCopy) {");
});

// ADR 0577: a function no JS caller sees, called and never named as a value,
// is given what its calls give it: React's heap's `siftUp` reads
// `heap[index]` of the index `push` gave it, the heap's length before its
// `push`. A `push` makes a length one more, a `pop` one fewer.
test("a function only calls reach is what its calls give it", async () => {
  const dir = fixture("only-called");
  writeFileSync(join(dir, "lib.rs"), `pub fn push(heap: &mut Vec<u32>, n: u32) {
    let index = heap.len();
    heap.push(n);
    sift_up(heap, index);
}

fn sift_up(heap: &mut [u32], i: usize) {
    let mut index = i;
    while index > 0 {
        let parent = (index - 1) >> 1;
        if heap[parent] > heap[index] {
            heap.swap(parent, index);
            index = parent;
        } else {
            return;
        }
    }
}

pub fn guarded(items: &[u32], i: usize) -> u32 {
    if i < items.len() { item(items, i) } else { 0 }
}

fn item(items: &[u32], i: usize) -> u32 {
    items[i]
}

pub fn mixed(items: &[u32], i: usize) -> u32 {
    let first = if i < items.len() { other(items, i) } else { 0 };
    first ^ other(items, i)
}

fn other(items: &[u32], i: usize) -> u32 {
    items[i]
}

pub fn doubled(items: &[u32]) -> (u32, Vec<u32>) {
    (double(3), items.iter().copied().map(double).collect())
}

fn double(n: u32) -> u32 {
    n * 2
}

pub fn half(n: u32) -> u32 {
    n - 1
}

pub fn halved() -> u32 {
    half(5)
}

pub fn counted() -> u32 {
    count(10, 0)
}

fn count(n: u32, total: u32) -> u32 {
    if n == 0 { total } else { count(n - 1, total + 1) }
}

pub fn pushed(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    v.push(7);
    v[n]
}

pub fn popped(v: &mut Vec<u32>) -> u32 {
    let n = v.len();
    if n == 0 {
        return 0;
    }
    v.pop();
    v[n - 1]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("    const parent = (index - 1) >>> 1;\n    if (heap[parent] > heap[index]) {\n      [heap[parent], heap[index]] = [heap[index], heap[parent]];\n");
  expect(js).toContain("function item(items, i) {\n  return items[i];\n}");
  // Called where nothing bounds it too, named as a value, or a JS caller's.
  expect(js).toContain("function other(items, i) {\n  return $index(items, i);\n}");
  expect(js).toContain("function double(n) {\n  return (n * 2) >>> 0;\n}");
  expect(js).toContain("export function half(n) {\n  return (n - 1) >>> 0;\n}");
  // Its own calls' too, until what it's given holds.
  expect(js).toContain("count(n - 1, (total + 1) >>> 0)");
  expect(js).toContain("  v.push(7);\n  return v[n];\n");
  expect(js).toContain("return $index(v, n - 1);");
  const lib = await import(join(dir, "lib.js"));
  const heap: number[] = [];
  for (const n of [5, 3, 8, 1]) lib.push(heap, n);
  expect(heap[0]).toBe(1);
  expect([lib.guarded([4, 5], 1), lib.guarded([4], 3), lib.counted(), lib.halved(), lib.pushed([1, 2])]).toEqual([5, 0, 10, 4, 7]);
  expect(lib.doubled([1, 2])).toEqual([6, [2, 4]]);
  expect(lib.half(0)).toBe(4294967295);
  expect(() => lib.mixed([1], 4)).toThrow("index out of bounds");
  expect(() => lib.popped([1])).toThrow("index out of bounds");
});

// ADR 0602: a binding taken as a `fn` is its JS function, read where it's
// taken, as a Rust `fn` is fixed once taken: React's scheduler keeps
// `const localSetImmediate = setImmediate`, which a polyfill set later
// doesn't change. Handed to JS's `map` itself, it's still an arrow of its
// own parameters, which JS gives no index.
test("a binding taken as a fn is its JS function, read where it's taken", async () => {
  const dir = fixture("binding-fn-pointers");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "rustJsHook")]
fn hook(n: f64) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "parseInt")]
fn parse_int(text: &str) -> f64 {
    unreachable!()
}

#[cfg_attr(rust_js, rust_js::link_name = "rustJsReset")]
#[cfg_attr(rust_js, rust_js::nullable(to))]
fn reset(to: Option<f64>) -> f64 {
    unreachable!()
}

thread_local! {
    // Captured, in case a polyfill overrides it.
    static LOCAL_HOOK: fn(f64) -> f64 = hook;
    // Its \`None\` given as \`null\`, as its call gives it: an arrow.
    static LOCAL_RESET: fn(Option<f64>) -> f64 = reset;
}

pub fn reset_to(to: Option<f64>) -> f64 {
    LOCAL_RESET.with(|reset| reset(to))
}

pub fn called(n: f64) -> f64 {
    LOCAL_HOOK.with(|hook| hook(n))
}

pub fn parsed() -> Vec<f64> {
    let parse: fn(&str) -> f64 = parse_int;
    vec!["10", "10"].into_iter().map(parse).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const localHook = rustJsHook;");
  expect(js).toContain("export const resetTo = (to) => rustJsReset(to ?? null);");
  const globals = globalThis as Record<string, unknown>;
  globals.rustJsHook = (n: number) => n + 1;
  globals.rustJsReset = (to: number | null) => (to === null ? -1 : to);
  const lib = await import(join(dir, "lib.js"));
  globals.rustJsHook = () => -1;
  expect([lib.called(1), lib.parsed(), lib.resetTo(undefined), lib.resetTo(4)]).toEqual([2, [10, 10], -1, 4]);
});

// A closure copies a variable set again only where it may be set after the
// closure is made: one set before it is read as it is, as react.dev's
// LoadingOverlay reads `fadeTimeout` in its cleanup. One set after, or in
// a loop around it, is copied where it's made, and so is one the closure changes.
test("a closure copies only what may be set after it's made", async () => {
  const dir = fixture("captured-after");
  writeFileSync(join(dir, "lib.rs"), `pub fn before(start: u32) -> impl Fn() -> u32 {
    let mut n = start;
    if n > 3 {
        n = 3;
    }
    move || n + 1
}
pub fn after(start: u32) -> u32 {
    let mut n = start;
    let f = move || n + 1;
    n = 10;
    f() + n
}
pub fn looped(xs: &[u32]) -> u32 {
    let mut n = 0;
    let mut fs = Vec::new();
    for x in xs {
        n = *x;
        let f = move || n;
        fs.push(f);
    }
    fs.iter().map(|f| f()).sum()
}
pub fn counted(start: u32) -> u32 {
    let mut n = start;
    let mut bump = move || {
        n += 1;
        n
    };
    bump();
    bump();
    n
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  const body = (name: string) => js.slice(js.indexOf(`function ${name}(`), js.indexOf("\n}\n", js.indexOf(`function ${name}(`)));
  expect(body("before")).not.toContain("n$1");
  expect(body("after")).toContain("const captured = n;");
  expect(body("looped")).toContain("const captured = n;");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.before(7)(), lib.after(1), lib.looped([4, 5]), lib.counted(5)]).toEqual([4, 12, 9, 5]);
});

// ADR 0308: a function written in a function's body is that body's, where
// it's written, as react.dev's useSandpackLint has `isReactRuleError` in
// its callback: a function declaration, which its block may call before
// it, as Rust's may. One a static's initializer calls stays the module's,
// where that initializer is.
test("a function written in a body is a local function there", async () => {
  const dir = fixture("local-functions");
  writeFileSync(join(dir, "lib.rs"), `pub fn kept(errors: Vec<Option<u32>>) -> (Vec<Option<u32>>, u32) {
    let early = twice(1);
    fn twice(n: u32) -> u32 {
        n * 2
    }
    let kept = errors
        .into_iter()
        .filter(|e| {
            fn present(e: &Option<u32>) -> bool {
                e.is_some()
            }
            present(e)
        })
        .collect();
    (kept, early)
}

pub fn started() -> u32 {
    fn start() -> u32 {
        5
    }
    thread_local! {
        static START: u32 = start();
    }
    START.with(|s| *s) + start()
}

mod util {
    pub fn three() -> u32 {
        3
    }
}

struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {}
}

pub fn nested() -> u32 {
    fn outer() -> u32 {
        fn inner() -> u32 {
            4
        }
        inner()
    }
    outer()
}

pub fn listed(n: u32) -> Vec<u32> {
    fn list_of(n: u32) -> Vec<u32> {
        let mut list = Vec::new();
        list.push(n);
        list
    }
    list_of(n)
}

pub fn guarded() -> u32 {
    let first = later();
    let _noisy = Noisy;
    fn later() -> u32 {
        util::three() + 4
    }
    first + later()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function kept(errors) {\n  const early = twice(1);\n  function twice(n) {");
  expect(js).toContain("(e) => {\n    function present(e) {");
  expect(js).toContain("\nfunction start() {");
  const lib = await import(join(dir, "lib.js"));
  // Made readable as a module's is: a `let mut` nothing sets again, a `const`.
  expect(js).toContain("  function listOf(n) {\n    const list = [];\n    list.push(n);");
  // Called before a destructor's `try`, it's written before it.
  expect(js).toContain("  function later() {\n    return (three() + 4) >>> 0;\n  }\n  try {\n    return (first + later()) >>> 0;\n  } finally {\n    noisyDrop_drop({ value: _noisy });");
  // One in another's body is a declaration there too.
  expect(js).toContain("  function outer() {\n    function inner() {");
  expect([lib.kept([1, undefined, 3]), lib.started(), lib.guarded(), lib.nested()]).toEqual([[[1, 3], 2], 10, 14, 4]);
});

// A `Copy` value a function gives, a variable of its own: handed over, as
// nothing reads it after, uncopied; a copy where what it was read from goes on.
test("a variable a function gives is handed over uncopied", async () => {
  const dir = fixture("returned-uncopied");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct P { pub x: i32 }

pub fn moved(x: i32, dx: i32) -> P {
    let mut p = P { x };
    p.x += dx;
    p
}

pub fn twice(p: &mut P) -> P {
    let q = *p;
    p.x += 1;
    q
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("  p.x = (p.x + dx) | 0;\n  return p;\n}");
  expect(js).toContain("  const q = { ...p };\n  p.x = (p.x + 1) | 0;\n  return q;");
  const lib = await import(join(dir, "lib.js"));
  const d = { x: 10 };
  const e = lib.twice(d);
  d.x += 100;
  expect([lib.moved(1, 2), d.x, e.x]).toEqual([{ x: 3 }, 111, 10]);
});

// A long function, each call a block of its MIR: lowered block by block,
// as deep as it's long, on a stack grown as rustc grows its own.
test("a long function is lowered block by block", async () => {
  const dir = fixture("long-function");
  const pushes = Array.from({ length: 3000 }, (_, i) => `    v.push(n + ${i});`).join("\n");
  writeFileSync(join(dir, "lib.rs"), `pub fn long(n: u32) -> Vec<u32> {\n    let mut v = Vec::new();\n${pushes}\n    v\n}\n`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const items = lib.long(1);
  expect([items.length, items[0], items[2999]]).toEqual([3000, 1, 3000]);
});

test("a closure's last `_` parameters are left out", async () => {
  const dir = fixture("mir-closure-wild-params");
  writeFileSync(join(dir, "lib.rs"), `pub fn ignored(n: u32) -> u32 {
    let f = |_: u32| n + 1;
    let g = |a: u32, _: &str| a * 2;
    f(1) + g(2, "x")
}
pub struct Loud(pub u32);
impl Drop for Loud {
    fn drop(&mut self) {}
}
pub fn dropped(n: u32) -> u32 {
    let h = |_: Loud| n + 1;
    h(Loud(3))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(`const f = () => (n + 1) >>> 0;`);
  expect(js).toContain(`const g = (a) => (a * 2) >>> 0;`);
  // What it's given and drops, it's given.
  expect(js).toMatch(/const h = \(_\d*\) =>/);
});

test("a closure from MIR names its `&x` parameter and knows the `bool`s it captures", async () => {
  const dir = fixture("mir-closure-captured-bools");
  writeFileSync(join(dir, "lib.rs"), `pub fn flags(force: bool, items: &[u32]) -> Vec<bool> {
    items.iter().map(|&n| force || n > 2).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  return items.map((n) => force || n > 2);\n");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.flags(false, [1, 3]), lib.flags(true, [1])]).toEqual([[false, true], [true]]);
});

test("a closure from MIR captures a variable itself where nothing changes it after", async () => {
  const dir = fixture("mir-captured-after");
  writeFileSync(join(dir, "lib.rs"), `pub fn last(n: u32, hover: &dyn Fn(&dyn Fn() -> u32)) {
    let mut index = 0;
    for _ in 0..n {
        index += 1;
    }
    hover(&move || index);
}
pub fn each(n: u32) -> Vec<u32> {
    let mut made: Vec<Box<dyn Fn() -> u32>> = Vec::new();
    let mut index = 0;
    while index < n {
        made.push(Box::new(move || index));
        index += 1;
    }
    made.iter().map(|f| f()).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  hover(() => index);\n");
  const lib = await import(join(dir, "mir.js"));
  let seen = 0;
  lib.last(3, (f: () => number) => { seen = f(); });
  expect([seen, lib.each(3)]).toEqual([3, [0, 1, 2]]);
});

test("a call from MIR is read in place past an object of constants before it", async () => {
  const dir = fixture("mir-past-constant-objects");
  writeFileSync(join(dir, "lib.rs"), `pub struct Colors { pub accent: &'static str, pub base: &'static str }
pub struct Font { pub body: String, pub mono: String, pub size: &'static str }
pub struct Theme { pub colors: Colors, pub font: Font }
fn joined(parts: &[&str]) -> String { parts.join(", ") }
fn unquoted(s: &str) -> String { s.replace('"', "") }
pub fn theme() -> Theme {
    Theme {
        colors: Colors { accent: "inherit", base: "inherit" },
        font: Font { body: unquoted(&joined(&["a", "b"])), mono: unquoted(&joined(&["c"])), size: "14px" },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('    font: { body: unquoted(joined(["a", "b"])), mono: unquoted(joined(["c"])), size: "14px" },\n');
});

// A temporary MIR makes of what calls, given to a builtin's function,
// `Object.fromEntries(_5)`, is read in place: `Object`'s members never
// change, so reading one first is the same, as react.dev's RSC template
// makes its files an object.
test("a call's temporary from MIR is read in place by a builtin's function", async () => {
  const dir = fixture("builtin-callee-inline");
  writeFileSync(join(dir, "lib.rs"), `use js::dict;
pub fn files(code: String, other: String) -> &'static js::Dict<String> {
    dict::from_entries(vec![
        ("/index.js".to_string(), code.to_uppercase()),
        ("/other.js".to_string(), other),
    ])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return { "/index.js": code.toUpperCase(), "/other.js": other };\n');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.files("a", "b")).toEqual({ "/index.js": "A", "/other.js": "b" });
});

// A chain read past the alias MIR makes of what its callback borrows
// mutably, `const list$1 = list;`: reading a variable nothing sets again is
// the same before the chain is made or after, so it's the array's own
// `forEach`, as react.dev's toCommaSeparatedList steps its items.
test("a chain is read in place past a closure's mutable borrow", async () => {
  const dir = fixture("chain-past-borrow");
  writeFileSync(join(dir, "lib.rs"), `pub fn list(array: &[String]) -> Vec<String> {
    let mut list: Vec<String> = Vec::new();
    array.iter().enumerate().for_each(|(index, item)| {
        if index > 0 {
            list.push(format!("{index}{item}"));
        }
    });
    list
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("  array.forEach((item, index) => {\n");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.list(["a", "b", "c"])).toEqual(["1b", "2c"]);
});

// ADR 0279, of a function item given as the callback, `filter(is_rule)`, as
// react.dev's useSandpackLint has it: the crate's function itself; a JS
// function's still takes the item alone.
test("a crate function given as a callback is passed as itself", async () => {
  const dir = fixture("fn-item-callback");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::link_name = "parseFloat")] #[allow(unused_variables)] fn parse_float(text: &str) -> f64 { unreachable!() }
pub struct E {
    pub rule: Option<String>,
}
fn is_rule(e: &&E) -> bool {
    e.rule.is_some()
}
fn twice(x: i32) -> i32 {
    x * 2
}
pub fn kept(errors: Vec<&E>) -> usize {
    errors.into_iter().filter(is_rule).count()
}
pub fn doubled(v: Vec<i32>) -> Vec<i32> {
    v.into_iter().map(twice).collect()
}
pub fn parsed(v: Vec<&str>) -> Vec<f64> {
    v.into_iter().map(parse_float).collect()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain(".filter(isRule)");
  expect(js).toContain("v.map(twice)");
  expect(js).toContain("v.map((text) => parseFloat(text))");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.kept([{ rule: "a" }, { rule: undefined }])).toBe(1);
  expect(lib.doubled([1, 2])).toEqual([2, 4]);
  expect(lib.parsed(["1", "2"])).toEqual([1, 2]);
});

// ADR 0308: a function written in a body, a closure's too, is declared
// there, before what's written after it, as react.dev's useSandpackLint has
// `isReactRuleError` in its linter's callback.
test("a function written in a closure is declared where it's written", async () => {
  const dir = fixture("written-fn-place");
  writeFileSync(join(dir, "lib.rs"), `pub fn counted(xs: Vec<i32>, m: i32) -> usize {
    let k = m + 1;
    fn small(x: i32) -> bool {
        x < 0
    }
    let f = move || {
        let n = xs.len();
        fn big(x: &&i32) -> bool {
            **x > 1
        }
        xs.iter().filter(big).count() + n
    };
    if small(k) { 0 } else { f() }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toMatch(/const k = .*\n  function small\(x\)/);
  expect(js).toMatch(/const n = xs\.length;\n    function big\(x\)/);
  const lib = await import(join(dir, "mir.js"));
  expect([lib.counted([1, 2, 3], 0), lib.counted([1, 2, 3], -2)]).toEqual([5, 0]);
});

// An `async` closure's `|_|`, which takes nothing it reads, takes nothing
// in JS either, `async () => ..`, as react.dev's PageHeading has its copy
// button's handler.
test("an async closure's ignored argument is no parameter", async () => {
  const dir = fixture("async-wild");
  writeFileSync(join(dir, "lib.rs"), `pub async fn run(f: impl AsyncFn(u32) -> u32) -> u32 {
    f(2).await
}
pub async fn made() -> u32 {
    run(async move |_| 1).await + run(async move |x| x).await
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("run(async () => 1)");
  const lib = await import(join(dir, "mir.js"));
  expect(await lib.made()).toBe(3);
});

// `mem::take(&mut current)` of a variable takes it and gives it a new
// value, `const old = current; current = [];`, not `$take(current, [])` of
// the object, a closure's `&mut` capture of one too, which is the
// variable; `mem::swap` of two, each given the other's.
test("taking a variable assigns the variable", async () => {
  const dir = fixture("take-variable");
  writeFileSync(join(dir, "lib.rs"), `pub fn direct(items: &[u32]) -> Vec<Vec<u32>> {
    let mut all = Vec::new();
    let mut current = Vec::new();
    for &item in items {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    }
    all
}
pub fn captured(items: &[u32]) -> Vec<Vec<u32>> {
    let mut all = Vec::new();
    let mut current = Vec::new();
    items.iter().for_each(|&item| {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    });
    all
}
pub fn swapped(mut a: Vec<u32>, mut b: Vec<u32>) -> (Vec<u32>, Vec<u32>) {
    std::mem::swap(&mut a, &mut b);
    (a, b)
}
pub fn half(mut a: Vec<u32>, b: &mut Vec<u32>) -> Vec<u32> {
    std::mem::swap(&mut a, b);
    a
}
pub fn kept(items: &[u32]) -> (Vec<Vec<u32>>, Vec<u32>) {
    let mut all = Vec::new();
    let mut current = Vec::new();
    items.iter().for_each(|&item| {
        if item == 0 {
            all.push(std::mem::take(&mut current));
        } else {
            current.push(item);
        }
    });
    (all, current)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).not.toContain("$take(");
  expect(js).toContain("  const t = a;\n  a = b;\n  b = t;");
  expect(js).toContain("$exchange(a, b);");
  expect(js).toContain("      const old = current;\n      current = [];\n      all.push(old);");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.direct([1, 2, 0, 3, 0]), lib.captured([1, 0, 0])]).toEqual([[[1, 2], [3]], [[1], []]]);
  expect(lib.kept([1, 0, 2, 3])).toEqual([[[1]], [2, 3]]);
});

// A value made in each branch of a `try`, a match of what a call gives,
// and given to a variable, `children = match .. {}`, is given to the
// variable in each, not to a temporary assigned after, as react.dev's
// Console shows what's logged.
test("a value a try makes in each branch is the variable's there", async () => {
  const dir = fixture("try-assigned");
  writeFileSync(join(dir, "lib.rs"), `use js::{Unknown, json, object};
pub fn shown(msg: Option<&Unknown>, tagged: bool) -> Option<String> {
    let children;
    if tagged {
        children = Some("tag".to_string());
    } else {
        children = match json::stringify_with(&msg, None, 2) {
            Ok(json) => json,
            Err(_) => match object::to_string(msg) {
                Ok(tag) => Some(tag),
                Err(_) => Some(format!("[{}]", js::type_of(msg))),
            },
        };
    }
    children
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("    try {\n      children = JSON.stringify(msg, null, 2);");
  const lib = await import(join(dir, "mir.js"));
  const cyclic: any = {};
  cyclic.self = cyclic;
  expect([lib.shown({ a: 1 }, false), lib.shown(cyclic, false), lib.shown(undefined, true)]).toEqual(['{\n  "a": 1\n}', "[object Object]", "tag"]);
});

// What MIR assigns once on each way, `classNameToken(if .. { "string" } else
// { "static" })`'s argument, isn't a variable the program names: it doesn't
// read alike, so what's made before it in a `vec!` stays where it's written,
// as react.dev's CodeBlock's `getSyntaxHighlight` has its specs, not held in
// `const`s first.
test("a vec's specs from MIR stay in place past a branch's temporary", async () => {
  const dir = fixture("specs-in-place");
  writeFileSync(join(dir, "lib.rs"), `use std::any::Any; use std::marker::PhantomData; use js::JsObject;
pub struct Syntax { pub string: String } pub struct Theme { pub syntax: Syntax }
pub struct Tag(PhantomData<JsObject>);
pub struct HighlightStyle(PhantomData<JsObject>);
#[cfg_attr(rust_js, rust_js::link_name = "@codemirror/language#HighlightStyle.define")]
#[allow(unused_variables)]
fn define(specs: Vec<&dyn Any>) -> &'static HighlightStyle {
    unreachable!()
}
mod tags {
    use super::Tag;

    unsafe extern "Rust" {
        #[link_name = "@lezer/highlight#tags.link"]
        pub safe static link: &'static Tag;
        #[link_name = "@lezer/highlight#tags.emphasis"]
        pub safe static emphasis: &'static Tag;
        #[link_name = "@lezer/highlight#tags.strong"]
        pub safe static strong: &'static Tag;
        #[link_name = "@lezer/highlight#tags.keyword"]
        pub safe static keyword: &'static Tag;
        #[link_name = "@lezer/highlight#tags.atom"]
        pub safe static atom: &'static Tag;
        #[link_name = "@lezer/highlight#tags.number"]
        pub safe static number: &'static Tag;
        #[link_name = "@lezer/highlight#tags.bool"]
        pub safe static bool: &'static Tag;
        #[link_name = "@lezer/highlight#tags.tagName"]
        pub safe static tagName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.variableName"]
        pub safe static variableName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.propertyName"]
        pub safe static propertyName: &'static Tag;
        #[link_name = "@lezer/highlight#tags.literal"]
        pub safe static literal: &'static Tag;
        #[link_name = "@lezer/highlight#tags.inserted"]
        pub safe static inserted: &'static Tag;
        #[link_name = "@lezer/highlight#tags.punctuation"]
        pub safe static punctuation: &'static Tag;
        #[link_name = "@lezer/highlight#tags.comment"]
        pub safe static comment: &'static Tag;
        #[link_name = "@lezer/highlight#tags.quote"]
        pub safe static quote: &'static Tag;
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.standard")]
    #[allow(unused_variables)]
    pub fn standard(tag: &Tag) -> &'static Tag {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.function")]
    #[allow(unused_variables)]
    pub fn function(tag: &Tag) -> &'static Tag {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "@lezer/highlight#tags.definition")]
    #[allow(unused_variables)]
    pub fn definition(tag: &Tag) -> &'static Tag {
        unreachable!()
    }
}

/// A spec's tag, or several.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Tags {
    One(&'static Tag),
    Many(Vec<&'static Tag>),
}

struct TextDecoratorStyle {
    tag: Tags,
    textdecorator: &'static str,
}

struct FontStyleStyle {
    tag: Tags,
    font_style: &'static str,
}

struct FontWeightStyle {
    tag: Tags,
    font_weight: &'static str,
}

struct ClassStyle {
    tag: Tags,
    class: String,
}
fn classNameToken(name: &str) -> String {
    format!("sp-syntax-{name}")
}

pub fn getSyntaxHighlight(theme: &Theme) -> &'static HighlightStyle {
    define(vec![
        &TextDecoratorStyle {
            tag: Tags::One(tags::link),
            textdecorator: "underline",
        },
        &FontStyleStyle {
            tag: Tags::One(tags::emphasis),
            font_style: "italic",
        },
        &FontWeightStyle {
            tag: Tags::One(tags::strong),
            font_weight: "bold",
        },
        &ClassStyle {
            tag: Tags::One(tags::keyword),
            class: classNameToken("keyword"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::atom, tags::number, tags::bool]),
            class: classNameToken("static"),
        },
        &ClassStyle {
            tag: Tags::One(tags::standard(tags::tagName)),
            class: classNameToken("tag"),
        },
        &ClassStyle {
            tag: Tags::One(tags::variableName),
            class: classNameToken("plain"),
        },
        &ClassStyle {
            // Highlight function call
            tag: Tags::One(tags::function(tags::variableName)),
            class: classNameToken("definition"),
        },
        &ClassStyle {
            // Highlight function definition differently (eg: functional component def in React)
            tag: Tags::Many(vec![
                tags::definition(tags::function(tags::variableName)),
                tags::tagName,
            ]),
            class: classNameToken("definition"),
        },
        &ClassStyle {
            tag: Tags::One(tags::propertyName),
            class: classNameToken("property"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::literal, tags::inserted]),
            class: classNameToken(if !theme.syntax.string.is_empty() {
                "string"
            } else {
                "static"
            }),
        },
        &ClassStyle {
            tag: Tags::One(tags::punctuation),
            class: classNameToken("punctuation"),
        },
        &ClassStyle {
            tag: Tags::Many(vec![tags::comment, tags::quote]),
            class: classNameToken("comment"),
        },
    ])
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "mir.js"), "utf8")).toContain('    { tag: tags.keyword, class: classNameToken("keyword") },\n');
});

// What rustc promotes in generic code it can't evaluate, `&size_of::<T>()`,
// `&None`, and a closure that captures nothing: made where they're used.
test("promoted values of generic code are made where they're used", async () => {
  const dir = fixture("promoted-generic");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
fn size_ref<T>() -> usize {
    let s: &usize = &std::mem::size_of::<T>();
    *s
}
pub fn sizes() -> Vec<usize> {
    vec![size_ref::<u8>(), size_ref::<u64>(), size_ref::<(u8, u32)>()]
}
fn none_ref<T: Clone>(v: Option<T>) -> Option<T> {
    let n: &Option<T> = &None;
    if v.is_some() { v } else { n.clone() }
}
pub fn nones() -> (Option<u32>, Option<u32>) {
    (none_ref(Some(3)), none_ref(None))
}
thread_local! {
    static LAST: Cell<u32> = const { Cell::new(0) };
}
fn apply(f: &dyn Fn(u32), n: u32) {
    f(n)
}
pub fn last() -> u32 {
    apply(&|n| LAST.with(|l| l.set(n)), 7);
    LAST.with(|l| l.get())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.sizes(), lib.nones(), lib.last()]).toEqual([[1, 8, 8], [3, undefined], 7]);
});
