// What the compiler writes of an impl's dictionary (ADRs 0049, 0592): its
// module's `const`, read as it is, made before what its module's load runs
// reads it, and where an import cycle could reach it before it's made.
// Each test is a program of its own, compiled and run.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCompiler, compiler, fixture, run } from "./support";

beforeAll(buildCompiler, 600_000);

// ADR 0592: an impl's dictionary that takes nothing is its module's
// \`const\`, \`export const circleShape = { area: circleShape_area }\`, read as
// it is; one generic code is given its parameters' a function, its cache a
// \`const\` too. No \`var\`. Its module's own load-time code reads it after
// it's made; another module's that imports it and that it imports may load
// first, so that cycle is refused, but for a thread-local made where it's
// first used (ADR 0037, amended), whose load runs nothing.
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
  // A thread-local whose initializer calls the crate's `total` is made where
  // it's first used, its dictionary read then (ADR 0037, amended).
  expect(js).toContain("const FIRST = $local(() => ({ value: total([[3]], squareShape) }));");
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

  // `counts`' thread-local is made where it's first used, so its load runs
  // nothing of `shapes`': the cycle compiles, and reads the dictionary once
  // both have loaded (ADR 0037, amended), where it was refused.
  run([compiler, join(cycle, "lib.rs"), "-o", join(cycle, "lib.js")]);
  expect((await import(join(cycle, "counts.js"))).first()).toBe(4);

  // One a binding makes is made where its module loads, as JS writes it
  // (ADR 0037, amended): that load runs code, so the cycle is refused.
  const loaded = fixture("dictionary-in-a-load-cycle-refused");
  writeFileSync(join(loaded, "lib.rs"), `pub trait Shape {
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

    #[cfg_attr(rust_js, rust_js::link_name = "tick")]
    fn tick() -> u32 {
        unreachable!()
    }

    pub const SCALE: u32 = 1;

    thread_local! {
        static FIRST: Cell<u32> = Cell::new(tick());
    }

    pub fn first() -> u32 {
        FIRST.get() + super::shapes::total(&[super::shapes::Square(2)])
    }
}
`);
  const refused = Bun.spawnSync([compiler, join(loaded, "lib.rs"), "-o", join(loaded, "lib.js")]);
  expect(refused.exitCode).not.toBe(0);
  expect(refused.stderr.toString()).toContain("this impl's dictionary, made where `shapes` loads, in an import cycle with `counts`, which runs code where it loads");

  // So is one whose cycle is through a JS file of the program's own that
  // `shapes` binds, which imports `counts` (ADR 0031, amended).
  const through = fixture("dictionary-in-a-load-cycle-through-js");
  writeFileSync(join(through, "lib.rs"), `pub trait Shape {
    fn area(&self) -> u32;
}

pub mod shapes {
    pub struct Square(pub u32);

    #[cfg_attr(rust_js, rust_js::link_name = "./scale.js#scale")]
    fn scale() -> u32 {
        unreachable!()
    }

    impl super::Shape for Square {
        fn area(&self) -> u32 {
            self.0 * self.0 * scale()
        }
    }

    pub fn total<S: super::Shape>(shapes: &[S]) -> u32 {
        shapes.iter().map(|s| s.area()).sum()
    }
}

pub mod counts {
    use std::cell::Cell;

    #[cfg_attr(rust_js, rust_js::link_name = "tick")]
    fn tick() -> u32 {
        unreachable!()
    }

    thread_local! {
        static FIRST: Cell<u32> = Cell::new(tick());
    }

    pub fn first() -> u32 {
        FIRST.get() + super::shapes::total(&[super::shapes::Square(2)])
    }
}
`);
  writeFileSync(join(through, "scale.js"), "import {first} from './counts.js';\n\nexport const scale = () => (first ? 1 : 0);\n");
  const cyclic = Bun.spawnSync([compiler, join(through, "lib.rs"), "-o", join(through, "lib.js")]);
  expect(cyclic.exitCode).not.toBe(0);
  expect(cyclic.stderr.toString()).toContain("this impl's dictionary, made where `shapes` loads, in an import cycle with `counts`, which runs code where it loads");
});

// ADR 0864, amended: a `PartialEq` whose `eq` is `ptr::eq(self, other)`
// alone, whatever its `Hash`, is `===`: `a != b` of two `Option<Rc<_>>` is
// `a != b`, and generic code is given `===`. A library writes its
// dictionary for its consumers, which no code of its own reads, so no
// load cycle reaches it, as React's update queue and hidden context
// import each other.
test("an identity PartialEq is ===, and its dictionary no load cycle reaches", async () => {
  const program = `pub mod queue {
    use std::rc::Rc;

    pub struct Update {
        pub lane: u32,
    }

    // An update is itself: two are equal only where they're one.
    impl PartialEq for Update {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }

    pub fn differ(a: &Option<Rc<Update>>, b: &Option<Rc<Update>>) -> bool {
        a != b
    }

    pub fn same(a: &Update, b: &Update) -> bool {
        a == b && super::hidden::has(&[a], b)
    }

    pub fn values(a: Update, b: Update) -> bool {
        a == b
    }
}

pub mod hidden {
    use std::cell::Cell;
    use std::rc::Rc;

    #[cfg_attr(rust_js, rust_js::link_name = "tick")]
    fn tick() -> u32 {
        unreachable!()
    }

    thread_local! {
        static FIRST: Cell<u32> = Cell::new(tick());
    }

    pub fn has<T: PartialEq>(items: &[T], item: T) -> bool {
        items.iter().any(|x| *x == item)
    }

    pub fn first() -> String {
        let update = Rc::new(super::queue::Update { lane: FIRST.get() });
        let other = Rc::new(super::queue::Update { lane: update.lane });
        format!(
            "{} {} {} {}",
            super::queue::differ(&Some(Rc::clone(&update)), &Some(Rc::clone(&update))),
            super::queue::differ(&Some(Rc::clone(&update)), &Some(other)),
            super::queue::differ(&None, &Some(Rc::clone(&update))),
            super::queue::same(&update, &update),
        )
    }
}
`;
  const dir = fixture("identity-eq-in-a-load-cycle");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const queue = readFileSync(join(dir, "queue.js"), "utf8");
  expect(queue).toContain("return a != b;");
  expect(queue).toContain("return a === b && has([a], b, { eq: (a, b) => a === b });");
  expect(queue).toContain("export function values(a, b) {\n  return a === b;\n}");
  expect(queue).not.toContain("PartialEq");
  (globalThis as { tick?: () => number }).tick = () => 3;
  expect((await import(join(dir, "hidden.js"))).first()).toBe("false true true true");

  const library = fixture("identity-eq-in-a-library-load-cycle");
  writeFileSync(join(library, "lib.rs"), program);
  run([compiler, join(library, "lib.rs"), "-o", join(library, "lib.js"), "--library", "--manifest", join(library, "m.json")]);
  expect(readFileSync(join(library, "queue.js"), "utf8")).toContain("export const updatePartialEq = { eq: updatePartialEq_eq };");
});
