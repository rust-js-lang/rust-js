// A type no other crate can make, `sealed` (ADR 1187), is branded where
// Rust makes it and checked by its brand where JS gives it (#2403 step 2c,
// ADR 1196): JS's own object of its shape, `{ celsius: -500 }`, is refused.
// The guard: every way Rust makes one, a literal, a constructor, a derived
// `Default` or `Clone`, a struct update, a copy, a `const`, a generic's,
// a value of no size a consumer's copy of a library's code makes,
// each given to JS every way it leaves, then given back, which a brand
// missed would refuse.
import { beforeAll, expect, test } from "bun:test";
import { cpSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { stdDir } from "./packages-of";
import { compileJs, compileJsLibrary } from "./programs";
import { buildSerde } from "./serde-crate";
import { compiler, fixture, run } from "./support";

const source = `use std::rc::Rc;

pub struct Email(String);

#[derive(Clone, Default)]
pub struct Temperature {
    celsius: f64,
}

#[derive(Clone, Copy)]
pub struct Point {
    pub x: u32,
    hidden: u32,
}

pub struct Id<T> {
    raw: u32,
    of: std::marker::PhantomData<T>,
}

#[repr(transparent)]
pub struct Celsius(f64);

pub const ORIGIN: Point = Point { x: 0, hidden: 7 };

pub fn email(text: &str) -> Email {
    Email(text.to_string())
}

pub fn domain(email: &Email) -> String {
    email.0.split('@').nth(1).unwrap_or("").to_string()
}

pub fn temperature(celsius: f64) -> Temperature {
    Temperature { celsius }
}

pub fn celsius(t: &Temperature) -> f64 {
    t.celsius
}

pub fn chilled() -> Temperature {
    Temperature::default()
}

pub fn copied(t: &Temperature) -> Temperature {
    t.clone()
}

pub fn warmer(t: &Temperature) -> Temperature {
    Temperature { celsius: t.celsius + 1.0, ..t.clone() }
}

pub fn point(x: u32) -> Point {
    Point { x, hidden: x }
}

pub fn moved(p: Point) -> Point {
    let mut q = p;
    q.x += 1;
    q
}

pub fn origin() -> Point {
    ORIGIN
}

pub fn x(p: &Point) -> u32 {
    p.x + p.hidden - p.hidden
}

pub fn id(raw: u32) -> Id<String> {
    Id { raw, of: std::marker::PhantomData }
}

pub fn raw(id: &Id<String>) -> u32 {
    id.raw
}

pub fn emails(texts: Vec<String>) -> Vec<Email> {
    texts.into_iter().map(Email).collect()
}

pub fn count(emails: &[Email]) -> usize {
    emails.len()
}

pub fn maybe(text: &str) -> Option<Email> {
    (!text.is_empty()).then(|| Email(text.to_string()))
}

pub fn shared(text: &str) -> Rc<Email> {
    Rc::new(Email(text.to_string()))
}

pub fn shared_domain(email: Rc<Email>) -> String {
    domain(&email)
}

pub fn handed(text: &str, to: &dyn Fn(Email) -> usize) -> usize {
    to(Email(text.to_string()))
}

pub fn back(f: fn() -> Email) -> String {
    domain(&f())
}

pub fn celsius_of(c: f64) -> Celsius {
    Celsius(c)
}

pub struct Guard<'a> {
    count: &'a std::cell::Cell<u32>,
    by: u32,
}

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.count.set(self.count.get() + self.by);
    }
}

fn hold(guard: Guard<'_>) -> u32 {
    let seen = guard.count.get();
    drop(guard);
    seen
}

pub fn guarded(count: &std::cell::Cell<u32>) -> u32 {
    let guard = Guard { count, by: 2 };
    let seen = guard.count.get();
    drop(guard);
    // One made whole, given to a function of the crate's.
    seen + hold(Guard { count, by: 1 })
}

pub fn degrees(c: Celsius) -> f64 {
    c.0
}
`;

let lib: Record<string, (...args: unknown[]) => any>;
let js: string;

beforeAll(async () => {
  const dir = fixture("boundary-brands");
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  lib = await import(join(dir, "lib.js"));
  js = readFileSync(join(dir, "lib.js"), "utf8");
}, 600_000);

const thrown = (call: () => unknown) => {
  try {
    call();
  } catch (error) {
    return `${(error as Error).name}: ${(error as Error).message}`;
  }
  return "nothing thrown";
};

test("JS's own object of a sealed type's shape is refused, naming the type", () => {
  expect(thrown(() => lib.celsius!({ celsius: -500 }))).toBe("TypeError: rust-js: celsius(t): expected a Temperature Rust made, got an object");
  expect(thrown(() => lib.domain!(["not an email"]))).toBe("TypeError: rust-js: domain(email): expected an Email Rust made, got an array");
});

test("a literal's and a constructor's are taken back", () => {
  expect(lib.celsius!(lib.temperature!(21))).toBe(21);
  expect(lib.domain!(lib.email!("a@b.c"))).toBe("b.c");
});

test("a derived Default's, a Clone's and a struct update's are taken back", () => {
  expect(lib.celsius!(lib.chilled!())).toBe(0);
  const t = lib.temperature!(5);
  expect(lib.celsius!(lib.copied!(t))).toBe(5);
  expect(lib.celsius!(lib.warmer!(t))).toBe(6);
});

test("a copy's and a const's are taken back", () => {
  const p = lib.point!(3);
  expect(lib.x!(lib.moved!(p))).toBe(4);
  expect(lib.x!(p)).toBe(3);
  expect(lib.x!(lib.origin!())).toBe(0);
  expect(lib.x!(lib.ORIGIN)).toBe(0);
});

test("a generic's is branded per type", () => {
  expect(lib.raw!(lib.id!(9))).toBe(9);
  expect(thrown(() => lib.raw!({ raw: 9 }))).toBe("TypeError: rust-js: raw(id): expected an Id Rust made, got an object");
});

test("one in a Vec, an Option or an Rc is taken back", () => {
  expect(lib.count!(lib.emails!(["a@b", "c@d"]))).toBe(2);
  expect(thrown(() => lib.count!([lib.email!("a@b"), ["x"]]))).toBe("TypeError: rust-js: count(emails[1]): expected an Email Rust made, got an array");
  expect(lib.domain!(lib.maybe!("a@b"))).toBe("b");
  expect(lib.sharedDomain!(lib.shared!("a@b"))).toBe("b");
});

test("one Rust gives a callback, and one a callback gives back, are taken", () => {
  let held: unknown;
  expect(lib.handed!("a@b", (email: unknown) => ((held = email), 1))).toBe(1);
  expect(lib.domain!(held)).toBe("b");
  expect(lib.back!(() => lib.email!("x@y"))).toBe("y");
  expect(thrown(() => lib.back!(() => ["x@y"]))).toBe("TypeError: rust-js: back(f) returned an array, expected an Email Rust made");
});

test("a value that never leaves Rust, a guard holding a borrow, isn't branded", () => {
  expect(js).not.toContain("::Guard");
  expect(js).toContain("hold({ count, by: 1 })");
  expect(lib.guarded!({ value: 1 })).toBe(4);
});

test("a pub newtype of a number keeps its field check: no WeakSet holds a number", () => {
  expect(lib.degrees!(lib.celsiusOf!(21))).toBe(21);
  expect(lib.degrees!(21)).toBe(21);
  expect(js).not.toContain("::Celsius");
});

// A library's sealed type a consumer clones: the consumer's code makes the
// clone, on its copy of the runtime, and the library's entry checks it on
// its own, bundled beside it (#2333): one registry of brands, both copies'.
const library = `#[derive(Clone)]
pub struct Email(String);

pub fn make(text: &str) -> Email {
    Email(text.to_string())
}

pub fn domain(email: &Email) -> String {
    email.0.split('@').nth(1).unwrap_or("").to_string()
}
`;

test("a library's sealed type its consumer clones, on another copy of the runtime, is the library's to take", async () => {
  const dir = fixture("boundary-brands-copies");
  writeFileSync(join(dir, "sealed_lib.rs"), library);
  const lib = compileJsLibrary(join(dir, "sealed_lib.rs"), dir, "2024", "sealed_lib");
  if (!("rmeta" in lib)) throw new Error(lib.error);
  const copy = join(dir, "sealed_lib", "node_modules", "@rust-js", "std");
  mkdirSync(copy, { recursive: true });
  cpSync(stdDir, copy, { recursive: true, dereference: true });
  writeFileSync(
    join(dir, "app.rs"),
    "pub fn twin(email: &sealed_lib::Email) -> sealed_lib::Email {\n    email.clone()\n}\n\nfn main() {}\n",
  );
  const app = compileJs(join(dir, "app.rs"), dir, "2024", "binary", {}, ["--dependency", lib.manifest], ["--extern", `sealed_lib=${lib.rmeta}`, "-L", dir]);
  if (!("js" in app)) throw new Error(app.error);
  expect(readFileSync(app.js, "utf8")).toContain('$brand("sealed_lib#');
  const made = await import(join(dir, "sealed_lib", "lib.js"));
  const twinned = await import(app.js);
  expect(made.domain(twinned.twin(made.make("a@b.c")))).toBe("b.c");
  expect(thrown(() => made.domain(["a@b.c"]))).toBe("TypeError: rust-js: domain(email): expected an Email Rust made, got an array");
});

// A library's generic code its consumer compiles from its optimized MIR
// (ADR 0914), which writes no value of no size: `check`'s `_0 =
// Checked(())` is gone, and the consumer's copy makes the type's one value
// of its fields' (#1889), branded as Rust makes it.
const zeroSized = `/// Proof that a name was checked.
pub struct Checked(());

pub fn check<N: AsRef<str>>(name: N) -> Checked {
    assert!(!name.as_ref().is_empty(), "a name is never empty");
    Checked(())
}

pub fn greet(_proof: &Checked, name: &str) -> String {
    format!("hello, {name}")
}
`;

test("a library's sealed value of no size its generic code makes in a consumer's copy is the library's to take", async () => {
  const dir = fixture("boundary-brands-zero-sized");
  writeFileSync(join(dir, "sealed_lib.rs"), zeroSized);
  const lib = compileJsLibrary(join(dir, "sealed_lib.rs"), dir, "2024", "sealed_lib");
  if (!("rmeta" in lib)) throw new Error(lib.error);
  // A name the check drops, so the consumer compiles its own `check`.
  writeFileSync(
    join(dir, "app.rs"),
    `struct Name(String);

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Drop for Name {
    fn drop(&mut self) {
        println!("checked {}", self.0);
    }
}

pub fn checked(name: &str) -> sealed_lib::Checked {
    sealed_lib::check(Name(name.to_string()))
}

fn main() {}
`,
  );
  const app = compileJs(join(dir, "app.rs"), dir, "2024", "binary", {}, ["--dependency", lib.manifest], ["--extern", `sealed_lib=${lib.rmeta}`, "-L", dir]);
  if (!("js" in app)) throw new Error(app.error);
  const made = await import(join(dir, "sealed_lib", "lib.js"));
  const consumer = await import(app.js);
  expect(made.greet(consumer.checked("Ada"), "Ada")).toBe("hello, Ada");
  expect(thrown(() => made.greet([undefined], "Ada"))).toBe("TypeError: rust-js: greet(proof): expected a Checked Rust made, got an array");
});

test("two versions of one crate are two types, each its own brand", () => {
  const dir = fixture("boundary-brands-versions");
  writeFileSync(join(dir, "lib.rs"), library);
  const keys = ["one", "two"].map((version) => {
    const js = join(dir, version, "lib.js");
    run([compiler, join(dir, "lib.rs"), "-o", js, "--", "--crate-type=lib", "--crate-name=sealed_lib", `-Cmetadata=${version}`, "--edition=2024"]);
    return /\$brand\("([^"]+)"/.exec(readFileSync(js, "utf8"))![1];
  });
  expect(keys[0]).toStartWith("sealed_lib#");
  expect(keys[0]).not.toBe(keys[1]);
});

test("one serde reads is branded as Rust made it", async () => {
  buildSerde("rlib");
  const dir = fixture("boundary-brands-serde");
  writeFileSync(
    join(dir, "cases.rs"),
    `#[derive(serde::Deserialize)]
pub struct Temperature {
    celsius: f64,
}

pub fn read(text: &str) -> Temperature {
    serde_json::from_str(text).unwrap()
}

pub fn celsius(t: &Temperature) -> f64 {
    t.celsius
}
`,
  );
  run([compiler, join(dir, "cases.rs"), "-o", join(dir, "cases.js"), "--", ...buildSerde()]);
  const read = await import(join(dir, "cases.js"));
  expect(read.celsius(read.read('{"celsius": 21}'))).toBe(21);
  expect(thrown(() => read.celsius({ celsius: 21 }))).toBe("TypeError: rust-js: celsius(t): expected a Temperature Rust made, got an object");
}, 600_000);
