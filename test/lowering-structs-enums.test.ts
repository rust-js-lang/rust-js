// What the compiler writes of structs and enums: a struct update as a spread,
// flattened fields, discriminated unions, untagged enums, a unit struct's
// name, derived orders and trait constants. Each test is a program of its own,
// compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

// A trait's default constant of `Self`, in a generic impl's dictionary, is
// its initializer copied into the impl, as a default method is (ADR 0176):
// read on each use, `Self` the impl's type. In a library, every one is,
// never computed where no consumer reads it, as rustc computes one only
// where it's read: `Wrap`'s would divide by zero.
test("a trait's default constant of Self is its initializer, copied into the impl", async () => {
  const dir = fixture("default-const-of-self");
  const source = `pub trait K {
    const A: u32;
    const B: u32 = Self::A + 1;
    const PER: u32 = 360 / Self::A;
    const NAME: &'static str = "k";
}

pub struct W<T>(pub T);

impl<T: K> K for W<T> {
    const A: u32 = T::A;
}

impl K for u8 {
    const A: u32 = 1;
}

pub struct Zero;

impl K for Zero {
    const A: u32 = 0;
}

fn b<S: K>() -> u32 {
    S::B
}

pub fn f() -> u32 {
    b::<W<u8>>()
}
`;
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).f()).toBe(2);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "library", "lib.js"), "--library", "--manifest", join(dir, "library", "manifest.json")]);
  const library = readFileSync(join(dir, "library", "lib.js"), "utf8");
  expect(library).toContain("get PER() {");
  // A literal is the value itself.
  expect(library).toContain('NAME: "k",');
  expect((await import(join(dir, "library", "lib.js"))).f()).toBe(2);
});

// Trait defaults of each other are an error only where they're read, as
// rustc has it (its defaults-cyclic-pass): a default's own `Self::B` isn't
// generic code reading `B`, which would compute `()`'s, a cycle.
test("trait constants defaulting to each other compile where none is read", async () => {
  const dir = fixture("default-const-cycle");
  writeFileSync(join(dir, "lib.rs"), `pub trait Tr {
    const A: u8 = Self::B;
    const B: u8 = Self::A;
}

impl Tr for () {}

impl Tr for u8 {
    const A: u8 = 42;
}

pub fn read() -> (u8, u8) {
    (<u8 as Tr>::A, <u8 as Tr>::B)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect((await import(join(dir, "lib.js"))).read()).toEqual([42, 42]);
});

// A struct updated from one it owns, `..file`, is that one spread, then the
// fields named, as a reference's is (ADR 0250): `{ ...code, hidden: true }`,
// as react.dev's RSC template hides its files. One of a type changed in
// place elsewhere too, moved into the update, as react.dev's
// SandpackWithHTMLOutput passes its props on; a `Copy` one, still there,
// keeps its fields read.
test("a struct updated from one it owns is a spread", async () => {
  const dir = fixture("owned-spread");
  writeFileSync(join(dir, "lib.rs"), `pub struct File {
    pub code: String,
    pub hidden: Option<bool>,
    pub active: Option<bool>,
}

pub fn hide(file: File) -> File {
    File { hidden: Some(true), ..file }
}

// Changed in place elsewhere: moved into the update, nothing changes it.
pub fn show(file: &mut File) {
    file.hidden = None;
}

#[derive(Clone, Copy)]
pub struct Pos {
    pub x: u32,
    pub y: u32,
}

pub fn nudge(pos: &mut Pos) {
    pos.y += 1;
}

pub fn right(pos: Pos) -> Pos {
    Pos { x: 9, ..pos }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { ...file, hidden: true };");
  expect(js).toContain("return { x: 9, y: pos.y };");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.hide({ code: "c", hidden: false, active: true })).toEqual({ code: "c", hidden: true, active: true });
});

// ADR 0284: an enum tagged by a property of its own, `#[rust_js::tag =
// "status"]`, is a discriminated union, as TypeScript's and ReScript's
// `@tag` are: each variant an object of its name and its fields,
// `{ status: "fulfilled", value }`, as `Promise.allSettled` gives them.
test("an enum tagged by a property of its own is a discriminated union", async () => {
  const dir = fixture("tagged-enum");
  writeFileSync(join(dir, "lib.rs"), `// A name equal to another of any case: its own \`==\`.
#[derive(Clone, Debug)]
pub struct Name(pub String);

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
#[derive(Clone, Debug, PartialEq)]
pub enum Settled {
    #[cfg_attr(rust_js, rust_js::name = "named")]
    Named { name: Name },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Fulfilled { value: u32 },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Rejected { reason: String },
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending,
    #[cfg_attr(rust_js, rust_js::name = "measured")]
    Measured { values: Vec<f64> },
}

pub fn made(n: u32) -> Vec<Settled> {
    vec![Settled::Fulfilled { value: n }, Settled::Rejected { reason: "no".to_string() }, Settled::Pending]
}

pub fn told(s: &Settled) -> String {
    match s {
        Settled::Fulfilled { value } => format!("got {value}"),
        Settled::Rejected { reason } => format!("failed: {reason}"),
        Settled::Pending => "waiting".to_string(),
        Settled::Named { name } => name.0.clone(),
        Settled::Measured { values } => format!("{} values", values.len()),
    }
}

pub fn same(a: &Settled, b: &Settled) -> bool {
    a.clone() == *b
}

const WAITING: Settled = Settled::Pending;
const DONE: Settled = Settled::Fulfilled { value: 7 };

pub fn constants() -> (Settled, Settled) {
    (WAITING, DONE)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('{ status: "fulfilled", value: n }');
  expect(js).toContain('{ status: "pending" }');
  expect(js).toContain('if (s.status === "fulfilled") {');
  const lib = await import(join(dir, "lib.js"));
  expect(lib.constants()).toEqual([{ status: "pending" }, { status: "fulfilled", value: 7 }]);
  expect(lib.made(1)).toEqual([{ status: "fulfilled", value: 1 }, { status: "rejected", reason: "no" }, { status: "pending" }]);
  // What JS made, as Promise.allSettled does, is read as Rust's.
  expect([lib.told({ status: "fulfilled", value: 2 }), lib.told({ status: "rejected", reason: "x" }), lib.told({ status: "pending" })])
    .toEqual(["got 2", "failed: x", "waiting"]);
  expect([lib.same({ status: "fulfilled", value: 2 }, { status: "fulfilled", value: 2 }), lib.same({ status: "pending" }, { status: "rejected", reason: "x" })])
    .toEqual([true, false]);
  expect(lib.same({ status: "named", name: ["Ada"] }, { status: "named", name: ["ADA"] })).toBe(true);
  // Equality compares fields by the tag too, NaN unequal as Rust has it.
  expect([lib.same({ status: "measured", values: [1] }, { status: "measured", values: [1] }), lib.same({ status: "measured", values: [NaN] }, { status: "measured", values: [NaN] })])
    .toEqual([true, false]);
});

// ADR 0214: an untagged enum's `Dict` is a plain object, of no class: told
// apart as an object, as a parsed URL's query, text or parsed, is.
test("an untagged enum's dictionary is told apart as an object", async () => {
  const dir = fixture("untagged-dict");
  writeFileSync(join(dir, "lib.rs"), `use js::{Dict, dict};

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Query {
    Str(String),
    Parsed(&'static Dict<u32>),
}

pub fn told(query: Option<Query>) -> String {
    match query {
        Some(Query::Parsed(parsed)) => format!("parsed {}", dict::get(parsed, "a").copied().unwrap_or(0)),
        Some(Query::Str(text)) => format!("text {text}"),
        None => "none".to_string(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("instanceof Dict");
  const lib = await import(join(dir, "lib.js"));
  expect([lib.told("a=1"), lib.told({ a: 2 }), lib.told(undefined)]).toEqual(["text a=1", "parsed 2", "none"]);
});

// ADR 0284: a tag may be `true` or `false`, react-dom's `FormStatus`
// `{ pending: true, data, .. } | { pending: false, .. }`, as ReScript's
// `@as(true)` is.
test("a discriminated union's tag may be a boolean", async () => {
  const dir = fixture("tagged-bool");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::tag = "pending")]
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    #[cfg_attr(rust_js, rust_js::name = true)]
    Pending { data: String },
    #[cfg_attr(rust_js, rust_js::name = false)]
    Idle,
}

pub fn made(data: &str) -> Vec<Status> {
    vec![Status::Pending { data: data.to_string() }, Status::Idle]
}

pub fn told(s: &Status) -> String {
    match s {
        Status::Pending { data } => format!("sending {data}"),
        Status::Idle => "idle".to_string(),
    }
}

pub fn same(a: &Status, b: &Status) -> bool {
    a.clone() == *b
}

pub fn shown(s: &Status) -> String {
    format!("{s:?}")
}

const IDLE: Status = Status::Idle;

pub fn constant() -> Status {
    IDLE
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ pending: true, data }");
  expect(js).toContain("if (s.pending === true) {");
  const lib = await import(join(dir, "lib.js"));
  expect(lib.made("a")).toEqual([{ pending: true, data: "a" }, { pending: false }]);
  expect(lib.constant()).toEqual({ pending: false });
  // What JS made, as useFormStatus does, is read as Rust's.
  expect([lib.told({ pending: true, data: "b" }), lib.told({ pending: false, data: null })]).toEqual(["sending b", "idle"]);
  expect([lib.same({ pending: true, data: "b" }, { pending: true, data: "b" }), lib.same({ pending: false }, { pending: true, data: "b" })])
    .toEqual([true, false]);
  expect(lib.shown({ pending: false })).toBe("Idle");
});

// ADR 0250: a struct update of a base nothing changes is that base spread,
// `{ ...attrs, title }`: what the base doesn't hold stays out, where a
// field read of it each would be there.
test("a struct update of a base nothing changes is the base spread", async () => {
  const dir = fixture("struct-update-spread");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub struct Attrs<'a> {
    pub title: Option<&'a str>,
    pub rel: Option<&'a str>,
    pub id: Option<&'a str>,
}
pub struct Link<'a> {
    pub attrs: Attrs<'a>,
}
pub fn retitled<'a>(attrs: Attrs<'a>, title: &'a str) -> Attrs<'a> {
    Attrs { title: Some(title), ..attrs }
}
pub fn reread<'a>(link: &Link<'a>, title: &'a str) -> Attrs<'a> {
    Attrs { title: Some(title), ..link.attrs }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return { ...attrs, title };");
  const lib = await import(join(dir, "mir.js"));
  expect(Object.keys(lib.retitled({ id: "a" }, "t"))).toEqual(["id", "title"]);
  expect(lib.reread({ attrs: { rel: "r" } }, "t")).toEqual({ rel: "r", title: "t" });
});

// ADR 0204: a struct with flattened fields made outside JSX is one object of
// their fields, `{ ...template, ...files }`.
test("a struct of flattened fields made outside JSX is one object", async () => {
  const dir = fixture("flattened-made");
  writeFileSync(join(dir, "lib.rs"), `pub struct Inner {
    pub a: u32,
}
pub struct Files<'a> {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub template: &'a Inner,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub files: &'a js::Unknown,
}
pub fn made<'a>(template: &'a Inner, files: &'a js::Unknown) -> Files<'a> {
    Files { template, files }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  expect(lib.made({ a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
});

// ADR 0214: an untagged enum only made, never told apart, may have variants
// of one kind, as Next.js's `getStaticProps` gives `{ props }` or
// `{ notFound: true }`, react.dev's errors page's: each is its payload.
test("an untagged enum only made may hold objects in two variants", async () => {
  const dir = fixture("untagged-objects");
  writeFileSync(join(dir, "lib.rs"), `pub struct Found {
    pub props: u32,
}

pub struct Missing {
    pub not_found: bool,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Page {
    Found(Found),
    Missing(Missing),
}

pub fn page(code: u32) -> Page {
    if code == 0 { Page::Missing(Missing { not_found: true }) } else { Page::Found(Found { props: code }) }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("{ notFound: true }");
  expect(js).toContain("{ props: code }");
  const { page } = await import(join(dir, "lib.js"));
  expect([page(0), page(3)]).toEqual([{ notFound: true }, { props: 3 }]);
});

// ADR 0264: a fieldless variant is its name (ADR 0013), so a `match` giving
// each variant its own name is what's matched, and a function that gives
// back what it's given, its argument: `/images/og-${section}.png`, as
// react.dev's Page writes it. One giving another name is a conditional.
test("an enum's own names are the enum", async () => {
  const dir = fixture("enum-names");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, PartialEq)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "blog")]
    Blog,
}

impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "blog",
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Blog => "news",
        }
    }
}

pub fn image(section: Section) -> String {
    format!("/images/og-{}.png", section.as_str())
}

pub fn title(section: Section) -> String {
    format!("{} page", section.heading())
}

// Mapped by it, an option's is the option, as react.dev's Page keys its
// SidebarNav, \`key={section}\`.
pub fn key(section: Option<Section>) -> Option<&'static str> {
    section.map(Section::as_str)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("asStr(section) {\n    return section;\n  }");
  expect(js).toContain("return `/images/og-${section}.png`;");
  expect(js).toContain("Section.heading(section)");
  expect(js).toContain("export function key(section) {\n  return section;\n}");
  const { image, title } = await import(join(dir, "lib.js"));
  expect([image("learn"), image("blog"), title("learn"), title("blog")]).toEqual(["/images/og-learn.png", "/images/og-blog.png", "learn page", "news page"]);
});

// A unit struct named `#[rust_js::name]` is that string, as a fieldless
// variant is (ADR 0013): `webapi`'s event names, `Click`, are types whose
// value is `"click"` (ADR 0223). One without a name holds nothing.
test("a named unit struct is its name", async () => {
  const dir = fixture("named-unit-struct");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::name = "click"]
pub struct Click;
pub struct Marker;
const CLICK: Click = Click;
fn pass<E>(event: E) -> E {
    event
}
pub fn click() -> Click {
    Click
}
pub fn passed() -> Click {
    pass(Click)
}
pub fn constant() -> Click {
    CLICK
}
pub fn marker() -> Marker {
    Marker
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return "click";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.click(), lib.passed(), lib.constant(), lib.marker()]).toEqual(["click", "click", "click", undefined]);
});

// ADR 0284: a discriminated union a binding knows only some of, Sandpack's
// messages say, ends in an `otherwise` variant: any object whose tag names
// none of the others, the object itself. A match tests the others' tags,
// whichever arm comes first, and `Other(o)` is `o`.
test("a discriminated union's otherwise variant is any other object", async () => {
  const dir = fixture("open-unions");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::tag = "type")]
pub enum Message {
    #[cfg_attr(rust_js, rust_js::name = "resize")]
    Resize { height: f64 },
    #[cfg_attr(rust_js, rust_js::name = "start")]
    Start {
        first_load: Option<bool>,
    },
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'static js::JsObject),
}

pub fn described(message: &Message) -> String {
    match message {
        Message::Resize { height } => format!("resize {height}"),
        Message::Start { first_load } => format!("start {}", first_load.unwrap_or(false)),
        Message::Other(_) => "other".to_string(),
    }
}

pub fn other(message: &Message) -> bool {
    matches!(message, Message::Other(_))
}

pub fn made(object: &'static js::JsObject) -> Message {
    Message::Other(object)
}

pub fn all_made(objects: Vec<&'static js::JsObject>) -> Vec<Message> {
    objects.into_iter().map(Message::Other).collect()
}
`);
  // From MIR too (ADR 0364).
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const lib = await import(join(dir, "mir.js"));
  const done = { type: "done", compilatonError: false };
  expect([lib.described({ type: "resize", height: 3 }), lib.described({ type: "start", firstLoad: true }), lib.described(done)])
    .toEqual(["resize 3", "start true", "other"]);
  expect([lib.other(done), lib.other({ type: "resize", height: 1 }), lib.made(done) === done, lib.allMade([done])[0] === done]).toEqual([true, false, true, true]);
});

// ADR 0592: an impl's dictionary that takes nothing is its module's
// \`const\`, \`export const circleShape = { area: circleShape_area }\`, read as
// it is; one generic code is given its parameters' a function, its cache a
// \`const\` too. No \`var\`. Its module's own load-time code reads it after
// it's made; another module's that imports it and that it imports may load
// first, so that cycle is refused.
test("a dictionary that takes nothing is its module's const", async () => {
  const dir = fixture("dictionary-consts");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;
use std::fmt;

pub trait Shape {
    fn area(&self) -> u32;
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

pub struct Wrapper<T>(pub T);

impl<T: fmt::Display> fmt::Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

pub fn total<S: Shape>(shapes: &[S]) -> u32 {
    shapes.iter().map(Shape::area).sum()
}

thread_local! {
    static FIRST: Cell<u32> = Cell::new(total(&[Square(3)]));
}

pub fn first() -> u32 {
    FIRST.get()
}

fn described<T: fmt::Display>(value: T) -> String {
    format!("shown {value}")
}

pub trait Bytes {
    fn bytes(&self) -> usize;
}

pub struct Slot<T>(pub T);

impl<T> Bytes for Slot<T> {
    fn bytes(&self) -> usize {
        std::mem::size_of::<T>()
    }
}

fn counted<B: Bytes>(slot: &B) -> usize {
    slot.bytes()
}

pub fn slots() -> usize {
    counted(&Slot(1u8)) + counted(&Slot(1u64))
}

pub fn shown(n: u32) -> String {
    described(Wrapper(n))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).not.toContain("var ");
  expect(js).toContain("export const squareShape = { area: squareShape_area };");
  expect(js).toContain("const $wrapperDisplay = new WeakMap();");
  expect(js.indexOf("export const squareShape")).toBeLessThan(js.indexOf("const FIRST = total([[3]], squareShape);"));
  const lib = await import(join(dir, "lib.js"));
  expect([lib.first(), lib.shown(5), lib.shown(6), lib.slots()]).toEqual([9, "shown [5]", "shown [6]", 9]);

  const cycle = fixture("dictionary-in-a-load-cycle");
  writeFileSync(join(cycle, "lib.rs"), `pub trait Shape {
    fn area(&self) -> u32;
}

pub mod shapes {
    pub struct Square(pub u32);

    impl super::Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0 * super::counts::SCALE
        }
    }

    pub fn total<S: super::Shape>(shapes: &[S]) -> u32 {
        shapes.iter().map(|s| s.area()).sum()
    }
}

pub mod counts {
    use std::cell::Cell;

    pub const SCALE: u32 = 1;

    thread_local! {
        static FIRST: Cell<u32> = Cell::new(super::shapes::total(&[super::shapes::Square(2)]));
    }

    pub fn first() -> u32 {
        FIRST.get()
    }
}
`);
  // Dictionaries and their caches alone run nothing where they're made: a
  // cycle of them compiles, as either's \`lib\` and \`iterator\` do.
  const quiet = fixture("dictionaries-in-a-cycle");
  writeFileSync(join(quiet, "lib.rs"), `use std::fmt;

pub mod wrapped {
    pub struct Wrapped<T>(pub T);

    impl<T: std::fmt::Display> std::fmt::Display for Wrapped<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "({})", super::Boxed(&self.0))
        }
    }
}

pub struct Boxed<T>(pub T);

impl<T: fmt::Display> fmt::Display for Boxed<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)
    }
}

pub fn shown(n: u32) -> String {
    wrapped::Wrapped(n).to_string()
}
`);
  run([compiler, join(quiet, "lib.rs"), "-o", join(quiet, "lib.js")]);
  expect((await import(join(quiet, "lib.js"))).shown(4)).toBe("([4])");

  const refused = Bun.spawnSync([compiler, join(cycle, "lib.rs"), "-o", join(cycle, "lib.js")]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("this impl's dictionary, made where `shapes` loads, in an import cycle with `counts`, which runs code where it loads");
});

// A struct with a flattened field, made outside JSX, is an object of its
// fields and the flattened one's: a struct made there inlined, another
// spread, `{ ...item, severity }`, as react.dev's runESLint writes
// `{...item, severity: severity[item.severity]}` (ADR 0204). A flattened
// field may be a reference to a struct.
test("a struct with a flattened field made outside JSX spreads it", async () => {
  const dir = fixture("flattened-made");
  writeFileSync(join(dir, "lib.rs"), `pub struct Message {
    pub line: f64,
    pub severity: f64,
}
pub struct Reported<'a> {
    #[rust_js::flatten]
    pub item: &'a Message,
    pub severity: &'static str,
}
pub fn reported(item: &Message) -> Reported<'_> {
    Reported {
        item,
        severity: if item.severity > 1.0 { "error" } else { "warning" },
    }
}
pub fn line(report: &Reported) -> f64 {
    report.item.line
}
pub struct Named {
    pub label: &'static str,
    #[rust_js::flatten]
    pub message: Message,
}
pub fn named() -> Named {
    Named {
        label: "x",
        message: Message { line: 1.0, severity: 2.0 },
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each across lines, as its literal is written (ADR 0835).
  expect(js).toContain('return {\n    ...item,\n    severity: item.severity > 1 ? "error" : "warning",\n  };');
  expect(js).toContain("return report.line;");
  expect(js).toContain('return {\n    label: "x",\n    line: 1,\n    severity: 2,\n  };');
  const lib = await import(join(dir, "lib.js"));
  const made = lib.reported({ line: 3, severity: 2 });
  expect([made, lib.line(made), lib.named()]).toEqual([{ line: 3, severity: "error" }, 3, { label: "x", line: 1, severity: 2 }]);
});

// `into()` an untagged enum, whose `from` is the variant of its argument,
// is the value (ADR 0214).
test("into an untagged enum is the value", async () => {
  const dir = fixture("into-untagged");
  writeFileSync(join(dir, "lib.rs"), `#[rust_js::untagged]
pub enum Src<'a> {
    Text(&'a str),
    Number(f64),
}

impl<'a> From<&'a str> for Src<'a> {
    fn from(text: &'a str) -> Self {
        Src::Text(text)
    }
}

pub fn text(s: &str) -> Src<'_> {
    s.into()
}

pub fn from(s: &str) -> Src<'_> {
    Src::from(s)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("export function text(s) {\n  return s;\n}");
  expect(js).toContain("export function from(s) {\n  return s;\n}");
});

// An untagged enum's variant of a fieldless enum is a string, its variant's
// name (ADR 0013), as next/font's `weight: "400" | Array<"400">` has it:
// made, and told from an array where it's matched.
test("an untagged enum's variant may hold a fieldless enum, a string", async () => {
  const dir = fixture("untagged-fieldless");
  writeFileSync(join(dir, "lib.rs"), `pub enum Weight {
    #[cfg_attr(rust_js, rust_js::name = "400")]
    Normal,
    #[cfg_attr(rust_js, rust_js::name = "700")]
    Bold,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Weights<'a> {
    One(Weight),
    Many(&'a [Weight]),
}

pub fn one() -> Weights<'static> {
    Weights::One(Weight::Bold)
}

pub fn many() -> Weights<'static> {
    Weights::Many(&[Weight::Normal, Weight::Bold])
}

pub fn count(weights: Weights) -> usize {
    match weights {
        Weights::One(_) => 1,
        Weights::Many(all) => all.len(),
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.one(), lib.many(), lib.count("700"), lib.count(["400", "700"])]).toEqual(["700", ["400", "700"], 1, 2]);
});

// An untagged enum's variant without fields is its name, a string literal,
// as TypeScript's `boolean | "blocking"` is: made, matched, compared and
// shown as one (ADR 0214). Beside a variant of any string, which JS can't
// tell it from, it's refused where it's matched.
test("an untagged enum's variant without fields is its name's string", async () => {
  const dir = fixture("untagged-literal");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fallback {
    Bool(bool),
    #[cfg_attr(rust_js, rust_js::name = "blocking")]
    Blocking,
}

pub fn fallback(b: bool) -> Fallback {
    if b { Fallback::Bool(true) } else { Fallback::Blocking }
}

pub fn describe(f: Fallback) -> String {
    match f {
        Fallback::Bool(b) => format!("bool {b}"),
        Fallback::Blocking => "blocking".to_string(),
    }
}

pub fn same(a: Fallback, b: Fallback) -> bool {
    a == b
}

pub fn shown(f: Fallback) -> String {
    format!("{f:?}")
}

pub fn is_blocking(f: Fallback) -> bool {
    matches!(f, Fallback::Blocking)
}

pub struct Loose(pub u32);

impl PartialEq for Loose {
    fn eq(&self, other: &Loose) -> bool {
        self.0 % 10 == other.0 % 10
    }
}

// Compared part by part, a part's own == among them.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(PartialEq)]
pub enum Choice {
    Loose(Loose),
    #[cfg_attr(rust_js, rust_js::name = "off")]
    Off,
}

pub fn same_choice(a: Choice, b: Choice) -> bool {
    a == b
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('  return b || "blocking";');
  expect(js).toContain('return f === "blocking";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.fallback(true), lib.fallback(false), lib.describe(true), lib.describe("blocking"), lib.same("blocking", "blocking"),
    lib.same(true, "blocking"), lib.same(true, true), lib.shown("blocking"), lib.shown(false), lib.isBlocking("blocking"), lib.isBlocking(false),
    lib.sameChoice("off", "off"), lib.sameChoice("off", [3]), lib.sameChoice([13], [3]), lib.sameChoice([3], "off")])
    .toEqual([true, "blocking", "bool true", "blocking", true, false, true, "Blocking", "Bool(false)", true, false, true, false, true, false]);
  writeFileSync(join(dir, "text.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Mode<'a> {
    Text(&'a str),
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
}

pub fn auto(m: Mode) -> bool {
    matches!(m, Mode::Auto)
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "text.rs"), "-o", join(dir, "text.js")], { cwd: root, stderr: "pipe" });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([false, expect.stringContaining("`Auto` and `Text` hold values of one kind")]);
});

// ADR 0676: an untagged enum's `#[rust_js::null]` variant is `null`, and
// holds nothing, as `js::Kind::Null` does.
test("an untagged enum's null variant holds nothing", () => {
  const dir = fixture("untagged-null-field");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Field {
    Count(f64),
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared(bool),
}

pub fn cleared(f: Field) -> bool {
    matches!(f, Field::Cleared(_))
}
`);
  const failed = Bun.spawnSync([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")], { cwd: root, stderr: "pipe" });
  expect([failed.exitCode === 0, failed.stderr.toString()]).toEqual([
    false,
    expect.stringContaining("an untagged enum's `#[rust_js::null]` variant, `Cleared`, is `null`, and holds nothing"),
  ]);
});

// ADR 0668: an `otherwise` variant is tested as the one kind `typeof` tells
// that the others aren't, `js::Kind::Object`'s `"object"`; a literal,
// `"auto"`, is one string, not every one, so beside it what's left is a
// string or an object, none of the others' tests.
test("an untagged enum's otherwise is the kind left, which a literal doesn't take", async () => {
  const dir = fixture("otherwise-kinds");
  writeFileSync(join(dir, "lib.rs"), `#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Setting<'a> {
    #[cfg_attr(rust_js, rust_js::name = "auto")]
    Auto,
    Size(f64),
    Big(i64),
    On(bool),
    Key(&'a js::Symbol),
    Call(&'a dyn Fn()),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Other(&'a js::Unknown),
}

pub fn other(setting: Setting) -> bool {
    matches!(setting, Setting::Other(_))
}

pub fn object(value: &js::Unknown) -> bool {
    matches!(js::classify(Some(value)), js::Kind::Object(_))
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('return typeof value === "object" && !Array.isArray(value);');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.other("auto"), lib.other("xyz"), lib.other({}), lib.other(1), lib.other(Symbol())]).toEqual([false, true, true, false, false]);
  expect([lib.object({}), lib.object([]), lib.object(Symbol()), lib.object(() => 1), lib.object("s")]).toEqual([true, false, false, false, false]);
});

// A derived order of an enum with fields: by variant, as declared, then
// the variant's fields in turn, as Rust's derive compares them; a float's
// \`NaN\` among them unordered.
test("a derived order of an enum with fields is by variant, then fields", async () => {
  const dir = fixture("derived-enum-order");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape { Circle { r: u32 }, Rect(u32, u32), Dot }
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Scaled { Dot, Scale(f64) }
pub fn sorted(mut v: Vec<Shape>) -> Vec<Shape> {
    v.sort();
    v
}
pub fn order(a: &Scaled, b: &Scaled) -> Option<std::cmp::Ordering> {
    a.partial_cmp(b)
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const lib = await import(join(dir, "mir.js"));
  const rect = (a: number, b: number) => ({ TAG: "Rect", _0: a, _1: b });
  const circle = (r: number) => ({ TAG: "Circle", r });
  expect(lib.sorted([rect(2, 3), "Dot", circle(2), rect(1, 9), circle(1), rect(2, 1)])).toEqual([circle(1), circle(2), rect(1, 9), rect(2, 1), rect(2, 3), "Dot"]);
  const scale = (x: number) => ({ TAG: "Scale", _0: x });
  expect([lib.order(scale(NaN), scale(1)), lib.order("Dot", scale(NaN)), lib.order(scale(2), scale(1))]).toEqual([undefined, -1, 1]);
});

// ADR 0264: a function giving each variant its own name gives what it's
// given: `section.as_str()` is `section`, as react.dev's Page names its
// section's image.
test("a call giving each variant its own name from MIR is what it's given", async () => {
  const dir = fixture("own-name-call");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy)]
pub enum Section {
    #[cfg_attr(rust_js, rust_js::name = "learn")]
    Learn,
    #[cfg_attr(rust_js, rust_js::name = "unknown")]
    Unknown,
}
impl Section {
    pub fn as_str(self) -> &'static str {
        match self {
            Section::Learn => "learn",
            Section::Unknown => "unknown",
        }
    }
}
pub fn image(section: Option<Section>) -> String {
    format!("/images/og-{}.png", section.unwrap_or(Section::Unknown).as_str())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('  return `/images/og-${section ?? "unknown"}.png`;\n');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.image("learn"), lib.image(undefined)]).toEqual(["/images/og-learn.png", "/images/og-unknown.png"]);
});

// A trait's default method that's a binding, `get ruleId`, called through
// a generic's bound, `error.rule_id()` of `E: RuleIdentified`: the
// binding of the value, `error.ruleId`, as react.dev's useSandpackLint
// keeps the errors of a rule.
test("a binding trait method called through a generic is the binding", async () => {
  const dir = fixture("generic-binding-method");
  writeFileSync(join(dir, "lib.rs"), `pub struct LintError {
    pub line: u32,
}
pub struct Diagnostic {
    pub from: f64,
}
trait RuleIdentified {
    #[cfg_attr(rust_js, rust_js::link_name = "get ruleId")]
    fn rule_id(&self) -> &Option<String> {
        unreachable!()
    }
}
impl RuleIdentified for LintError {}
impl RuleIdentified for Diagnostic {}
pub fn kept(errors: Vec<&'static LintError>, diags: Vec<&'static Diagnostic>) -> (usize, Vec<&'static Diagnostic>) {
    fn isReactRuleError<E: RuleIdentified>(error: &E) -> bool {
        error.rule_id().is_some()
    }
    let n = errors.into_iter().filter(|e| isReactRuleError(*e)).count();
    (n, diags.into_iter().filter(|d| isReactRuleError(*d)).collect())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain("return error.ruleId != null;");
  const lib = await import(join(dir, "mir.js"));
  expect(lib.kept([{ line: 1, ruleId: "a" }, { line: 2, ruleId: null }], [{ from: 1, ruleId: null }, { from: 2, ruleId: "b" }])).toEqual([1, [{ from: 2, ruleId: "b" }]]);
});

// ADR 0250: a struct update whose every field is given, `Hit { url, ..*item
// }`, still spreads its base, though it reads none of its fields: what JS
// gave holds more than Rust names, react.dev's Search gives DocSearch its
// hits' hierarchy back.
test("a struct update giving every field spreads its base from MIR", async () => {
  const dir = fixture("update-all-given");
  writeFileSync(join(dir, "lib.rs"), `pub struct Hit {
    pub url: String,
}
pub fn cleaned(items: Vec<&'static Hit>) -> Vec<Hit> {
    items.into_iter().map(|item| Hit { url: item.url.replace("#x", ""), ..*item }).collect()
}
pub fn moved(hit: Hit) -> Hit {
    Hit { url: hit.url.replace("#x", ""), ..hit }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('({ ...item, url: item.url.replaceAll("#x", "") })');
  expect(js).toContain('return { ...hit, url: hit.url.replaceAll("#x", "") };');
  const lib = await import(join(dir, "mir.js"));
  expect([lib.cleaned([{ url: "a#x", level: 1 }]), lib.moved({ url: "b#x", level: 2 })]).toEqual([[{ url: "a", level: 1 }], { url: "b", level: 2 }]);
});

// ADR 0250: a struct update that reads one field of its base, `Props {
// title, ..props }`, spreads it, as its THIR says it's an update: not
// `{ title, level: props.level }`, which drops what JS's object held
// besides: react.dev's SandpackWithHTMLOutput gives on its props. One that
// names each field, `level: a.level`, isn't.
test("a struct update reading one field of its base spreads it from MIR", async () => {
  const dir = fixture("update-one-read");
  writeFileSync(join(dir, "lib.rs"), `pub struct Props {
    pub title: String,
    pub level: u32,
}
pub fn retitled(props: Props) -> Props {
    Props { title: "x".to_string(), ..props }
}
pub fn copied(a: &Props) -> Props {
    Props { title: "y".to_string(), level: a.level }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  expect(js).toContain('return { ...props, title: "x" };');
  expect(js).toContain('return { title: "y", level: a.level };');
  const lib = await import(join(dir, "mir.js"));
  expect(lib.retitled({ title: "a", level: 1, extra: 2 })).toEqual({ title: "x", level: 1, extra: 2 });
});

// An update's derived `Default` base, `..Default::default()`, is each
// default the update reads, where it reads it, `{ size, name: "", tags: []
// }`, made without the base's `default()`, which a generic component's
// `children: C` would need a dictionary for.
test("an update's default base is the defaults it reads", async () => {
  const dir = fixture("default-base");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Default, Debug)]
pub struct Options {
    pub name: String,
    pub size: u32,
    pub tags: Vec<String>,
    pub label: Option<String>,
}
pub fn sized(size: u32) -> Options {
    Options { size, ..Default::default() }
}
pub fn named(name: String, size: u32) -> String {
    let o = Options { name, size, ..Default::default() };
    format!("{} {} {}", o.name, o.size, o.tags.len())
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "mir.js")]);
  const js = readFileSync(join(dir, "mir.js"), "utf8");
  // What's written, then what the base fills, as Rust evaluates them (ADR 0674).
  expect(js).toContain('return { size, name: "", tags: [] };');
  expect(js).toContain("const o = { name, size, tags: [] };");
  const lib = await import(join(dir, "mir.js"));
  expect([lib.sized(3), lib.named("a", 2)]).toEqual([{ name: "", size: 3, tags: [] }, "a 2 0"]);
});

// A `no_std` crate that names `alloc` its own way, hashbrown's `extern crate
// alloc as stdalloc`: rustc names std's items through it, and what rust-js
// knows of each is still std's, `Vec::capacity` and the allocator's.
test("std's items named through a crate's own name for alloc are std's", async () => {
  const dir = fixture("aliased-alloc");
  writeFileSync(join(dir, "lib.rs"), `#![no_std]
extern crate alloc as stdalloc;
use stdalloc::vec::Vec;
pub fn made(n: usize) -> usize {
    let mut v: Vec<u8> = Vec::with_capacity(n);
    v.push(1);
    v.capacity()
}
pub fn sized(n: usize) -> usize {
    let layout = stdalloc::alloc::Layout::from_size_align(n, 8).unwrap();
    unsafe {
        let p = stdalloc::alloc::alloc(layout);
        stdalloc::alloc::dealloc(p, layout);
    }
    layout.size()
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type", "lib", "--crate-name", "aliased"]);
  const lib = await import(join(dir, "lib.js"));
  expect([lib.made(4), lib.sized(16)]).toEqual([4, 16]);
});

// ADR 0674: a struct expression's object lists its fields as they're
// written, the order Rust evaluates them in, then those `..base` fills, as
// declared; a spread of the base before them, whose fields JS's after it
// override; a flattened field's own where it's written. The keys JS reads,
// `Object.keys`, are in that order, as React's `ConsolePatchingDev` writes
// the descriptors it gives `Object.defineProperties`.
test("a struct literal's keys are in the order written", async () => {
  const dir = fixture("fields-as-written");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Default)]
pub struct Logs {
    pub info: Option<u32>,
    pub log: Option<u32>,
    pub warn: Option<u32>,
}
pub fn reenabled(prev: u32) -> Logs {
    Logs { log: Some(prev), info: Some(prev + 1), warn: Some(prev + 2) }
}
pub fn updated(logs: Logs) -> Logs {
    Logs { warn: Some(1), log: Some(2), ..logs }
}
pub fn filled(warn: u32) -> Logs {
    Logs { warn: Some(warn), ..Default::default() }
}
pub struct Inner {
    pub a: u32,
    pub b: u32,
}
pub struct Outer<'a> {
    pub first: u32,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub inner: &'a Inner,
    pub last: u32,
}
pub fn flat(inner: &Inner) -> Outer<'_> {
    Outer { inner, last: 3, first: 1 }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("return { log: prev, info: (prev + 1) >>> 0, warn: (prev + 2) >>> 0 };");
  expect(js).toContain("return { ...logs, warn: 1, log: 2 };");
  expect(js).toContain("return { warn };");
  expect(js).toContain("return { ...inner, last: 3, first: 1 };");
  const lib = await import(join(dir, "lib.js"));
  expect(Object.keys(lib.reenabled(1))).toEqual(["log", "info", "warn"]);
  expect(lib.updated({ info: 7, log: 8, warn: 9 })).toEqual({ info: 7, log: 2, warn: 1 });
  expect(Object.keys(lib.flat({ a: 1, b: 2 }))).toEqual(["a", "b", "last", "first"]);
});

// ADR 0722: a fieldless enum under a `#[repr(..)]` of an integer is its
// number, `ArrayKind::Empty` `0`, as React's `const EMPTY_ARRAY = 0;` is;
// its `as` the value, its `{:?}` its name, a `.d.ts` its numbers. One
// without, or a variant named by `#[rust_js::name]`, its name, as ever.
test("a repr enum's variant is its number", async () => {
  const dir = fixture("repr-enum-numbers");
  writeFileSync(join(dir, "lib.rs"), `#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum ArrayKind {
    Empty = 0,
    Complex = 1,
    Primitive = 2,
    Entries = 3,
}

#[derive(Clone, Copy, Debug)]
#[repr(i32)]
pub enum Sparse {
    Low = -1,
    High = 10,
}

#[derive(Clone, Copy, Debug)]
#[repr(u64)]
pub enum Wide {
    Small = 3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plain {
    Empty,
}

pub fn kind(n: usize) -> ArrayKind {
    if n == 0 { ArrayKind::Empty } else { ArrayKind::Entries }
}

pub fn is_empty(kind: ArrayKind) -> bool {
    kind == ArrayKind::Empty
}

pub fn code(kind: ArrayKind) -> u8 {
    kind as u8
}

pub fn shown(kind: ArrayKind, sparse: Sparse) -> String {
    format!("{kind:?} {sparse:?}")
}

pub fn others(wide: Wide, plain: Plain) -> String {
    format!("{wide:?} {plain:?}")
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Cell<'a> {
    Kind(ArrayKind),
    Text(&'a str),
}

pub fn cell(cell: Cell) -> String {
    match cell {
        Cell::Kind(kind) => format!("{kind:?}"),
        Cell::Text(text) => text.to_string(),
    }
}

pub fn name(kind: ArrayKind) -> &'static str {
    match kind {
        ArrayKind::Empty => "Empty",
        ArrayKind::Complex => "Complex",
        ArrayKind::Primitive => "Primitive",
        ArrayKind::Entries => "Entries",
    }
}

pub fn named(n: usize) -> String {
    name(kind(n)).to_string()
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Each variant its module's `const`, read by its name (ADR 0722, amended).
  expect(js).toContain("export const Empty = 0;\nexport const Complex = 1;\nexport const Primitive = 2;\nexport const Entries = 3;\n");
  expect(js).toContain("export const Low = -1;\nexport const High = 10;\n");
  expect(js).toContain("return n === 0 ? Empty : Entries;");
  expect(js).toContain("return kind === Empty;");
  expect(js).toContain("export function code(kind) {\n  return kind;\n}");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export type ArrayKind = 0 | 1 | 2 | 3;");
  expect(declarations).toContain("export const Empty: 0;");
  expect(declarations).toContain("export const Low: -1;");
  expect(declarations).toContain('export type Plain = "Empty";');
  const lib = await import(join(dir, "lib.js"));
  expect([lib.kind(0), lib.kind(4), lib.isEmpty(0), lib.code(3)]).toEqual([0, 3, true, 3]);
  expect([lib.Empty, lib.Entries, lib.Low, lib.High]).toEqual([0, 3, -1, 10]);
  expect([lib.shown(2, -1), lib.shown(0, 10), lib.others("Small", "Empty")]).toEqual(["Primitive Low", "Empty High", "Small Empty"]);
  // A function giving each variant its name gives a name, not the number.
  expect([lib.name(0), lib.name(3), lib.named(0), lib.named(1)]).toEqual(["Empty", "Entries", "Empty", "Entries"]);
  // Shown by its number, an untagged enum's payload told as a number.
  expect(js).toContain('["Empty", "Complex", "Primitive", "Entries"][kind]');
  expect([lib.cell(3), lib.cell("x")]).toEqual(["Entries", "x"]);
});

// React's `ReactLazy` keeps a lazy component's status as module constants,
// `const Pending = 0;`, each read by its name: a `#[repr]` enum's variant
// is its module's `const`, as a `const` item is (ADRs 0031, 0722): tested,
// set, matched by a `switch`'s `case Pending:` and cast, the cast its
// number. Named apart as any of a module's names, two enums' `Pending`
// `Pending` and `Pending$1`; another module's imported by its name.
test("a repr enum's variants are its module's consts, each read by its name", async () => {
  const dir = fixture("repr-enum-consts");
  writeFileSync(join(dir, "lib.rs"), `use std::cell::Cell;

#[derive(Clone, Copy, PartialEq)]
#[repr(i8)]
enum Status {
    Uninitialized = -1,
    Pending = 0,
    Resolved = 1,
    Rejected = 2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u8)]
enum Request {
    Pending = 5,
    Done = 6,
}

pub struct Payload {
    status: Cell<Status>,
}

pub fn lazy() -> Payload {
    Payload { status: Cell::new(Status::Uninitialized) }
}

pub fn settle(payload: &Payload, ok: bool) -> i32 {
    if payload.status.get() == Status::Uninitialized {
        payload.status.set(Status::Pending);
    }
    if matches!(payload.status.get(), Status::Pending | Status::Uninitialized) {
        payload.status.set(if ok { Status::Resolved } else { Status::Rejected });
    }
    payload.status.get() as i32
}

pub fn describe(payload: &Payload) -> &'static str {
    match payload.status.get() {
        Status::Uninitialized => "uninitialized",
        Status::Pending => "pending",
        Status::Resolved => "resolved",
        Status::Rejected => "rejected",
    }
}

pub fn code(done: bool) -> i32 {
    let request = if done { Request::Done } else { Request::Pending };
    Status::Pending as i32 + request as i32
}

pub fn requests() -> String {
    format!("{:?}", [Some(Request::Pending), None])
}

pub mod theme {
    #[derive(Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mode {
        Light = 1,
        Dark = 2,
    }
}

pub fn mode(dark: bool) -> theme::Mode {
    if dark { theme::Mode::Dark } else { theme::Mode::Light }
}

mod motion {
    #[derive(Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub(crate) enum Speed {
        Slow = 1,
        Fast = 2,
    }

    pub(crate) fn speed(fast: bool) -> Speed {
        if fast { Speed::Fast } else { Speed::Slow }
    }
}

pub fn pace(fast: bool) -> &'static str {
    match motion::speed(fast) {
        motion::Speed::Slow => "slow",
        motion::Speed::Fast => "fast",
    }
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("const Uninitialized = -1;\nconst Pending = 0;\nconst Resolved = 1;\nconst Rejected = 2;\n");
  expect(js).toContain("const Pending$1 = 5;\nconst Done = 6;\n");
  expect(js).toContain('import { Dark, Light } from "./theme.js";');
  expect(js).toContain("return { status: Uninitialized };");
  expect(js).toContain("if (payload.status === Uninitialized) {\n    payload.status = Pending;\n  }");
  expect(js).toContain("if (payload.status === Pending || payload.status === Uninitialized) {\n    payload.status = ok ? Resolved : Rejected;\n  }");
  expect(js).toContain("case Uninitialized:");
  expect(js).toContain("case Rejected:");
  // A constant rustc made of one, `[Some(Request::Pending), None]`'s.
  expect(js).toContain("[Pending$1, undefined]");
  expect(js).toContain("const request = done ? Done : Pending$1;");
  expect(js).toContain("return dark ? Dark : Light;");
  expect(readFileSync(join(dir, "theme.js"), "utf8")).toContain("export const Light = 1;\nexport const Dark = 2;\n");
  // Another module's not `pub`, exported as it's read there; the last
  // arm's, whose test an exhaustive `match` needn't write, not imported.
  expect(js).toContain('import { Slow, speed } from "./motion.js";');
  expect(js).toContain('  if (speed(fast) === Slow) {\n    return "slow";\n  }\n  return "fast";\n');
  expect(readFileSync(join(dir, "motion.js"), "utf8")).toContain("export const Slow = 1;\nexport const Fast = 2;\n");
  const lib = await import(join(dir, "lib.js"));
  const [first, second] = [lib.lazy(), lib.lazy()];
  expect([lib.describe(first), lib.settle(first, true), lib.describe(first)]).toEqual(["uninitialized", 1, "resolved"]);
  expect([lib.settle(second, false), lib.describe(second)]).toEqual([2, "rejected"]);
  expect([lib.code(false), lib.code(true), lib.requests(), lib.mode(true), lib.mode(false)]).toEqual([5, 6, "[Some(Pending), None]", 2, 1]);
  expect([lib.pace(true), lib.pace(false)]).toEqual(["fast", "slow"]);
});

// ADR 0835: an object of a struct literal written across lines, a line
// ending after its `{`, as rustfmt writes one too long for a line, is on
// lines of its own, as Prettier keeps an object a person wrote so; one
// written on one line is on one where it fits, and so is an object no
// literal writes, a range's or a tuple variant's. A `const`'s or a
// `static`'s, rustc's value, is as its literal is written.
test("an object is across lines where its struct literal is written so", () => {
  const dir = fixture("objects-as-written");
  writeFileSync(join(dir, "lib.rs"), `pub struct Ref {
    pub kind: &'static str,
    pub render: u32,
}

pub enum Event {
    Moved { dx: i32, dy: i32 },
    Placed(u32),
}

pub fn element(render: u32) -> Ref {
    let element_type = Ref {
        kind: "forward_ref",
        render,
    };
    element_type
}

pub fn short(render: u32) -> Ref {
    Ref { kind: "short", render }
}

pub struct Described {
    pub description: &'static str,
}

pub fn described() -> Described {
    Described {
        description: "one field, too long for rustfmt's line",
    }
}

pub fn events() -> Vec<Event> {
    vec![
        Event::Moved { dx: 1, dy: -2 },
        Event::Moved {
            dx: 3,
            dy: 4,
        },
        Event::Placed(5),
    ]
}

pub fn updated(base: Ref) -> Ref {
    Ref {
        kind: "updated",
        ..base
    }
}

pub fn span(from_the_start: bool) -> std::ops::Range<u32> {
    (if from_the_start {
        0
    } else {
        1
    })..2
}

pub fn placed(from_the_start: bool) -> Event {
    Event::Placed(if from_the_start {
        0
    } else {
        1
    })
}

pub const DEFAULT: Ref = Ref {
    kind: "default",
    render: 0,
};

pub static ONE_LINE: Ref = Ref { kind: "static", render: 1 };
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const elementType = {\n    kind: "forward_ref",\n    render,\n  };');
  expect(js).toContain('return { kind: "short", render };');
  expect(js).toContain('return {\n    description: "one field, too long for rustfmt\'s line",\n  };');
  expect(js).toContain('{ TAG: "Moved", dx: 1, dy: -2 },\n    {\n      TAG: "Moved",\n      dx: 3,\n      dy: 4,\n    },\n    { TAG: "Placed", _0: 5 },');
  expect(js).toContain('return {\n    ...base,\n    kind: "updated",\n  };');
  expect(js).toContain("return { start: fromTheStart ? 0 : 1, end: 2 };");
  expect(js).toContain('return { TAG: "Placed", _0: fromTheStart ? 0 : 1 };');
  expect(js).toContain('export const DEFAULT = {\n  kind: "default",\n  render: 0,\n};');
  expect(js).toContain('export const ONE_LINE = { kind: "static", render: 1 };');
});
