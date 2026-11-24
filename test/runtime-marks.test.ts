// The marks the runtime sets on what it makes, which every copy of the
// runtime reads (ADR 1184): declared in `runtime/marks.js` alone, each a
// registry symbol, `Symbol.for("rust-js: ..")`. A bundled library and the
// app that uses it may each hold a copy, and a panic one raises is caught,
// downcast or hooked by the other: a mark one copy made by `Symbol(..)`
// is unseen by the other, a `String` payload read as a `&str`.

import { expect, test } from "bun:test";
import { cpSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { runSync } from "./child";
import { libraryCrate } from "./metamorphic";
import { stdDir } from "./packages-of";
import { agree, compileJs, compileJsLibrary, nativeFlags, runJs, runNative, type Run } from "./programs";
import { fixture, root } from "./support";

const runtime = join(root, "crates", "rust-js-program", "src", "runtime");

test("the runtime's marks are registry symbols, declared in marks.js alone", () => {
  const made: string[] = [];
  const registered: string[] = [];
  for (const file of readdirSync(runtime).filter((name) => name.endsWith(".js"))) {
    for (const line of readFileSync(join(runtime, file), "utf8").split("\n")) {
      const declared = /^const (\$\w+) = (Symbol(?:\.for)?)\(/.exec(line);
      if (declared === null) continue;
      const [, name, how] = declared;
      if (how === "Symbol") made.push(`${file} ${name}`);
      // `Symbol.dispose` where the host has none: the well-known symbol's
      // stand-in, not a mark of rust-js's.
      else if (!(file === "dropping.js" && name === "$disposeKey")) registered.push(`${file} ${name} ${line.includes('"rust-js: ') ? "named" : "unnamed"}`);
    }
  }
  expect(made).toEqual([]);
  expect(registered.sort()).toEqual(
    ["$arrayIterKey", "$cloneKey", "$countedKey", "$hookKey", "$infiniteAtKey", "$itemArray", "$itemIndex", "$partsOf", "$placeOfPart", "$raisedAt", "$slicedKey", "$stdoutKey", "$stringPayload"].map((name) => `marks.js ${name} named`),
  );
});

// `library` built as a library crate, `metamorph_lib`, and `program` that
// uses it: native Rust's run, and the JS's with the library on a copy of
// the runtime of its own, beside it, which Node resolves its `@rust-js/std`
// to, the app's the checkout's (#2292, #2333).
function twoCopies(name: string, library: string, program: string): { native: Run; ran: Run } {
  const dir = fixture(`runtime-marks-${name}`);
  const libraryFile = join(dir, "library.rs");
  writeFileSync(libraryFile, library);
  const programFile = join(dir, "program.rs");
  writeFileSync(programFile, program);
  const rlib = join(dir, `lib${libraryCrate}.rlib`);
  const built = runSync(["rustc", ...nativeFlags("2024"), "--crate-type=rlib", `--crate-name=${libraryCrate}`, libraryFile, "-o", rlib], root, 120_000);
  expect(built.stderr).toBe("");
  const native = runNative(programFile, dir, "2024", "host", ["--extern", `${libraryCrate}=${rlib}`]);
  if (typeof native === "string") throw new Error(native);
  const lib = compileJsLibrary(libraryFile, dir, "2024", libraryCrate);
  if (!("rmeta" in lib)) throw new Error(lib.error);
  const compiled = compileJs(programFile, dir, "2024", "binary", {}, ["--dependency", lib.manifest], ["--extern", `${libraryCrate}=${lib.rmeta}`, "-L", dir]);
  if (!("js" in compiled)) throw new Error(compiled.error);
  const copy = join(dir, libraryCrate, "node_modules", "@rust-js", "std");
  mkdirSync(copy, { recursive: true });
  cpSync(stdDir, copy, { recursive: true, dereference: true });
  return { native, ran: runJs(["node"], compiled.js, dir, "node") };
}

test("a panic raised by a library's copy of the runtime is its app's copy's: its payload, a String, downcasts, and where it was raised is read, as native Rust's", () => {
  const { native, ran } = twoCopies(
    "panic",
    'pub fn formatted(n: u32) -> u32 {\n    if n > 2 {\n        panic!("n = {n}");\n    }\n    n\n}\n\npub fn plain() {\n    panic!("plain");\n}\n',
    `use std::panic::catch_unwind;

fn main() {
    for n in [1, 3] {
        match catch_unwind(|| ${libraryCrate}::formatted(n)) {
            Ok(n) => println!("ok {n}"),
            Err(payload) => println!("{:?} {:?}", payload.downcast_ref::<String>(), payload.downcast_ref::<&str>()),
        }
    }
    let payload = catch_unwind(${libraryCrate}::plain).unwrap_err();
    println!("{:?} {:?}", payload.downcast_ref::<String>(), payload.downcast_ref::<&str>());
    ${libraryCrate}::plain();
}
`,
  );
  expect(native.stdout).toBe('ok 1\nSome("n = 3") None\nNone Some("plain")\n');
  // And where its last panic, uncaught, was raised, the library's line.
  expect(native.outcome).toEqual({ exit: 101 });
  expect(native.stderr).toMatch(/panicked at [^\n]*library\.rs:9:5:\nplain\n/);
  expect([ran.stdout, ran.outcome, ran.stderr]).toEqual([native.stdout, native.outcome, native.stderr]);
  expect(agree(ran, native)).toBe(true);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: rc.js \`$countedKey\`: an \`Rc\` of a class a library made, counted and dropped by its app, its \`Weak\` upgrading never after`, () => {
  const { native, ran } = twoCopies(
    "rc_class",
    `use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Node {
    pub name: String,
    pub weight: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Node {
    pub fn new(name: &str) -> Self {
        Node {
            name: name.to_string(),
            weight: 3,
        }
    }

    pub fn describe(&self) -> String {
        format!("{}:{}", self.name, self.weight)
    }
}

pub fn make(name: &str) -> Rc<Node> {
    Rc::new(Node::new(name))
}

pub fn count(node: &Rc<Node>) -> usize {
    Rc::strong_count(node)
}
`,
    `use std::rc::Rc;

fn main() {
    let a = metamorph_lib::make("a");
    let b = Rc::clone(&a);
    println!("{}", b.describe());
    println!("app counts {}", Rc::strong_count(&a));
    println!("dep counts {}", metamorph_lib::count(&a));
    let weak = Rc::downgrade(&a);
    drop(b);
    println!(
        "after drop {} {}",
        Rc::strong_count(&a),
        metamorph_lib::count(&a)
    );
    println!("weak {} {}", Rc::weak_count(&a), weak.upgrade().is_some());
    drop(a);
    println!("weak after {}", weak.upgrade().is_some());
}
`,
  );
  expect(native.stdout).toBe(`a:3
app counts 2
dep counts 2
after drop 1 1
weak 1 true
weak after false
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: iter.js \`$arrayIterKey\`: a library's slice and \`Vec\` iterators, their \`size_hint\` and steps read by its app`, () => {
  const { native, ran } = twoCopies(
    "slice_iter",
    `pub fn items(v: &[i32]) -> std::slice::Iter<'_, i32> {
    v.iter()
}

pub fn owned(v: Vec<String>) -> std::vec::IntoIter<String> {
    v.into_iter()
}
`,
    `fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let mut it = metamorph_lib::items(&v);
    it.next();
    println!("{} {:?}", it.len(), it.as_slice());
    println!("{:?}", it.next_back());
    println!("{:?}", it.clone().rev().collect::<Vec<_>>());
    println!("{:?} {:?}", it.nth(1), it.size_hint());
    let mut o = metamorph_lib::owned(vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
    ]);
    println!("{:?}", o.nth(1));
    println!("{:?} {:?}", o.as_slice(), o.len());
    println!("{:?}", o.next_back());
    println!("{:?}", o.last());
}
`,
  );
  expect(native.stdout).toBe(`4 [2, 3, 4, 5]
Some(5)
[4, 3, 2]
Some(3) (1, Some(1))
Some("b")
["c", "d"] 2
Some("d")
Some("c")
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: slice_ops.js \`$placeOfPart\`: a part of a slice a library made, its place compared and offset by its app`, () => {
  const { native, ran } = twoCopies(
    "part",
    `pub fn tail(v: &[i32]) -> &[i32] {
    &v[1..]
}

pub fn tail_mut(v: &mut [i32]) -> &mut [i32] {
    &mut v[1..]
}

pub fn offset(whole: &[i32], part: &[i32]) -> isize {
    unsafe { part.as_ptr().offset_from(whole.as_ptr()) }
}

pub fn first_eq(a: &[i32], b: &[i32]) -> bool {
    std::ptr::eq(&a[0], &b[0])
}
`,
    `fn main() {
    let v = vec![1, 2, 3, 4];
    let t = metamorph_lib::tail(&v);
    println!("{}", std::ptr::eq(&t[0], &v[1]));
    println!("{}", unsafe { t.as_ptr().offset_from(v.as_ptr()) });
    println!("{}", metamorph_lib::offset(&v, &v[2..]));
    println!("{}", metamorph_lib::first_eq(&v[1..], t));
    let mut m = vec![10, 20, 30];
    let tm = metamorph_lib::tail_mut(&mut m);
    let p = &mut tm[0] as *mut i32;
    unsafe { *p += 5 };
    println!("{:?}", m);
}
`,
  );
  expect(native.stdout).toBe(`true
1
2
true
[10, 25, 30]
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: chunks.js \`$partsOf\`: a library's \`Chunks\` and \`Windows\`, shown by its app`, () => {
  const { native, ran } = twoCopies(
    "chunks",
    `pub fn pairs(v: &[i32]) -> std::slice::Chunks<'_, i32> {
    v.chunks(2)
}

pub fn runs(v: &[i32]) -> std::slice::Windows<'_, i32> {
    v.windows(2)
}
`,
    `fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let mut c = metamorph_lib::pairs(&v);
    println!("{:?}", c);
    println!("{:?}", c.next());
    println!("{:?}", c);
    let d = c.clone();
    println!("{:?} {:?}", d, d.size_hint());
    let mut w = metamorph_lib::runs(&v);
    w.next();
    println!("{:?}", w);
    println!("{:?}", w.collect::<Vec<_>>());
}
`,
  );
  expect(native.stdout).toBe(`Chunks { v: [1, 2, 3, 4, 5], chunk_size: 2 }
Some([1, 2])
Chunks { v: [3, 4, 5], chunk_size: 2 }
Chunks { v: [3, 4, 5], chunk_size: 2 } (2, Some(2))
Windows { v: [2, 3, 4, 5], size: 2 }
[[2, 3], [3, 4], [4, 5]]
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: cloning.js \`$cloneKey\`: a library's closure, cloned by its app`, () => {
  const { native, ran } = twoCopies(
    "clone",
    `pub fn counter(start: u32) -> impl FnMut() -> u32 + Clone {
    let mut n = start;
    move || {
        n += 1;
        n
    }
}

pub fn tagged(tag: String) -> impl Fn(u32) -> String + Clone {
    move |n| format!("{tag}{n}")
}

pub fn cycled(v: &[u32]) -> std::iter::Cycle<std::slice::Iter<'_, u32>> {
    v.iter().cycle()
}
`,
    `fn main() {
    let mut a = metamorph_lib::counter(10);
    println!("{}", a());
    let mut b = a.clone();
    println!("{} {} {}", a(), b(), b());
    let t = metamorph_lib::tagged("x".to_string());
    let u = t.clone();
    println!("{} {}", t(1), u(2));
    let v = [1, 2, 3];
    let mut c = metamorph_lib::cycled(&v);
    c.next();
    let d = c.clone();
    println!(
        "{:?} {:?}",
        c.take(4).collect::<Vec<_>>(),
        d.take(4).collect::<Vec<_>>()
    );
}
`,
  );
  expect(native.stdout).toBe(`11
12 12 13
x1 x2
[2, 3, 1, 2] [2, 3, 1, 2]
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: mut_items.js \`$itemArray\` and \`$itemIndex\`: a pointer to an item of a library's \`IterMut\`, written through by its app`, () => {
  const { native, ran } = twoCopies(
    "items",
    `pub fn each_mut(v: &mut [u32]) -> std::slice::IterMut<'_, u32> {
    v.iter_mut()
}

pub fn bump(numbers: &mut [u32]) {
    for n in numbers.iter_mut() {
        let p = n as *mut u32;
        unsafe { *p += 100 };
    }
}
`,
    `fn main() {
    let mut w = [1u32, 2, 3];
    for n in metamorph_lib::each_mut(&mut w) {
        let p = n as *mut u32;
        unsafe { *p += 1 };
    }
    println!("{:?}", w);
    metamorph_lib::bump(&mut w);
    println!("{:?}", w);
}
`,
  );
  expect(native.stdout).toBe(`[2, 3, 4]
[102, 103, 104]
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

// No result found that a per-copy `$slicedKey` changes: without it a JS
// iterator made of the `IntoIter` takes its dictionary's steps, which pass
// over and drop what they pass in the same order (#2333's triage, ~15
// forms); held, as the rest are, so none of these marks goes back.
test(`a value a library's copy of the runtime made is its app's copy's, as native Rust's: closing.js \`$slicedKey\`: a library's generic code stepping its app's \`Vec\`'s \`IntoIter\``, () => {
  const { native, ran } = twoCopies(
    "sliced",
    `pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn nth_of<I: Iterator>(mut it: I, n: usize) -> Option<I::Item> {
    it.nth(n)
}

pub fn last_of<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

pub fn back_nth_of<I: DoubleEndedIterator>(mut it: I, n: usize) -> Option<I::Item> {
    let got = it.nth_back(n);
    println!("got back");
    got
}
`,
    `use metamorph_lib::Loud;

fn make() -> Vec<Loud> {
    (1..=5).map(Loud).collect()
}

fn main() {
    let a = metamorph_lib::nth_of(make().into_iter(), 2);
    println!("nth {:?}", a.as_ref().map(|l| l.0));
    drop(a);
    let b = metamorph_lib::last_of(make().into_iter());
    println!("last {:?}", b.as_ref().map(|l| l.0));
    drop(b);
    let c = metamorph_lib::back_nth_of(make().into_iter(), 1);
    println!("nth_back {:?}", c.as_ref().map(|l| l.0));
}
`,
  );
  expect(native.stdout).toBe(`drop 1
drop 2
drop 4
drop 5
nth Some(3)
drop 3
drop 1
drop 2
drop 3
drop 4
last Some(5)
drop 5
drop 5
got back
drop 1
drop 2
drop 3
nth_back Some(4)
drop 4
`);
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

// What stdout holds of a line left unended is every copy's (#2320): a
// library's copy holds its `print!`, its app's copy holds its own and ends
// the line, and natively one `LineWriter` held both, in the order written.
// Each copy's own queue wrote the app's first and the library's at exit.
test("what a library's copy of the runtime holds of a line its app's copy ends comes first, as native Rust's", () => {
  const { native, ran } = twoCopies(
    "stdout_held",
    'pub fn part(n: u32) {\n    print!("lib {n} ");\n}\n\npub fn line() {\n    println!("lib line");\n}\n',
    `fn main() {
    ${libraryCrate}::part(1);
    print!("app ");
    println!("end");
    ${libraryCrate}::part(2);
    ${libraryCrate}::line();
    print!("app ");
    ${libraryCrate}::part(3);
    print!("at exit");
}
`,
  );
  expect(native.stdout).toBe("lib 1 app end\nlib 2 lib line\napp lib 3 at exit");
  expect([ran.stdout, ran.outcome]).toEqual([native.stdout, native.outcome]);
});

test("a `Repeat` a library's copy of the runtime made, counted by its app's copy, names where the app counts it, as native Rust's", () => {
  // Where its `count` was called is held in `globalThis`, under a key every
  // copy reads (#2324, #2333's rule).
  const { native, ran } = twoCopies(
    "repeat",
    "pub fn forever(n: u32) -> std::iter::Repeat<u32> {\n    std::iter::repeat(n)\n}\n",
    `fn main() {
    println!("{}", metamorph_lib::forever(2).take(2).count());
    println!("{}", metamorph_lib::forever(1).count());
}
`,
  );
  expect(native.stdout).toBe("2\n");
  expect(native.outcome).toEqual({ exit: 101 });
  expect(native.stderr).toMatch(/panicked at [^\n]*program\.rs:3:46:\niterator is infinite\n/);
  expect(agree(ran, native)).toBe(true);
});
