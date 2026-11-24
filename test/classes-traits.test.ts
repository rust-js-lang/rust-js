// A trait of classes (rd283): the crate's trait each impl of which is a
// `#[rust_js::class]`'s. Each of its methods, an impl's or the trait's
// default, is each class's prototype's, a default compiled for each class
// with `Self` known: an ES class's its own copy in its body (#2495), a
// constructor function's one function assigned to each whose copy prints
// alike, as React DOM's ReactDOMRoot writes `ReactDOMHydrationRoot.prototype
// .render = ReactDOMRoot.prototype.render = function (children) { .. }`.
// Each test is a program of its own, compiled and run in Node.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const flags = (dir: string) => ["--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `rust` compiled in a fixture of `name`: its JS, its declarations, and its module. */
async function compiled(name: string, rust: string): Promise<{ js: string; ts: string; lib: Record<string, unknown> }> {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  run([compiler, ...flags(dir)]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  return { js, ts: readFileSync(join(dir, "lib.d.ts"), "utf8"), lib: await import(join(dir, "lib.js")) };
}

/** What the compiler says of `rust`, which it refuses. */
function refused(name: string, rust: string): string {
  const dir = fixture(name);
  writeFileSync(join(dir, "lib.rs"), rust);
  const ran = runSync([compiler, ...flags(dir)], root, 60_000);
  expect(ran.code).not.toBe(0);
  return ran.stderr;
}

const replaced = (name: string) => `
unsafe impl js::Defined for ${name} {}
impl js::Classed for ${name} {
    type Constructor = js::AnyConstructor;
}
`;

const roots = `use std::cell::RefCell;

/// What \`createRoot\` and \`hydrateRoot\` give.
pub trait RootType {
    fn internal_root(&self) -> &RefCell<Option<f64>>;

    fn render(&self, children: f64, #[cfg_attr(rust_js, rust_js::rest)] args: &[f64]) -> f64 {
        let Some(root) = *self.internal_root().borrow() else {
            panic!("Cannot update an unmounted root.");
        };
        root + children + args.len() as f64
    }

    fn unmount(&self) {
        self.internal_root().replace(None);
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactDOMRoot {
    #[cfg_attr(rust_js, rust_js::name = "_internalRoot", rust_js::nullable)]
    pub internal_root: RefCell<Option<f64>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactDOMRoot {
    pub fn new(root: f64) -> Self {
        ReactDOMRoot {
            internal_root: RefCell::new(Some(root)),
        }
    }
}

impl RootType for ReactDOMRoot {
    fn internal_root(&self) -> &RefCell<Option<f64>> {
        &self.internal_root
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactDOMHydrationRoot {
    #[cfg_attr(rust_js, rust_js::name = "_internalRoot", rust_js::nullable)]
    pub internal_root: RefCell<Option<f64>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactDOMHydrationRoot {
    pub fn new(root: f64) -> Self {
        ReactDOMHydrationRoot {
            internal_root: RefCell::new(Some(root)),
        }
    }
}

impl RootType for ReactDOMHydrationRoot {
    fn internal_root(&self) -> &RefCell<Option<f64>> {
        &self.internal_root
    }

    fn unmount(&self) {
        self.internal_root().replace(Some(0.0));
    }
}
`;

// Constructor functions, their prototypes replaced (#2495): one function
// each default prints alike, assigned to both, as upstream's; the one an
// impl writes its own, each class's.
test("a trait of classes' default that prints alike is one function of constructor functions' prototypes", async () => {
  const { js, lib } = await compiled(
    "class-traits-shared",
    `${roots}${replaced("ReactDOMRoot")}${replaced("ReactDOMHydrationRoot")}
js::on_load! {
    js::class::<ReactDOMRoot>().set_prototype(js::class::<ReactDOMRoot>().prototype());
    js::class::<ReactDOMHydrationRoot>().set_prototype(js::class::<ReactDOMHydrationRoot>().prototype());
}

pub fn render_both(a: &ReactDOMRoot, b: &ReactDOMHydrationRoot) -> f64 {
    a.render(1.0, &[]) + b.render(2.0, &[3.0])
}
`,
  );
  expect(js.match(/const root = this\._internalRoot;/g)?.length).toBe(1);
  expect(js).toContain("export function ReactDOMRoot(");
  expect(js).toContain("_internalRoot = null;");
  expect(js).toContain("_internalRoot = 0;");
  expect((lib.renderBoth as (a: unknown, b: unknown) => number)(new (lib.ReactDOMRoot as any)(10), new (lib.ReactDOMHydrationRoot as any)(20))).toBe(34);
});

test("a trait of classes' methods are each class's prototype's", async () => {
  const { js, ts, lib } = await compiled(
    "class-traits",
    `${roots}
pub fn render_both(a: &ReactDOMRoot, b: &ReactDOMHydrationRoot) -> f64 {
    a.render(1.0, &[]) + b.render(2.0, &[3.0])
}
`,
  );
  // Each ES class its own copy of the default, as Rust makes a provided
  // method each impl's: two functions no Rust can tell apart (#2495, ADR
  // 0262); the one an impl writes its own, each class's.
  expect(js.match(/^  render\(children, \.\.\.args\) \{$/gm)?.length).toBe(2);
  expect(js).toContain("export class ReactDOMRoot {");
  expect(js).toContain("  unmount() {\n    this._internalRoot = null;\n  }");
  expect(js).toContain("  unmount() {\n    this._internalRoot = 0;\n  }");
  expect(js).not.toContain("RootType");
  // The method that gives a field is that field, read in place, no
  // method of its own, its borrows never checked where none can meet
  // another, as an inherent method's (rd304).
  expect(js).toContain("{\n    const root = this._internalRoot;\n    if (root == null) {");
  expect(js).not.toMatch(/\binternalRoot\b/);
  expect(js).not.toContain("$fieldCell");
  expect(ts).not.toMatch(/\binternalRoot\b/);
  expect(js).toContain("return a.render(1) + b.render(2, 3);");
  expect(ts).toMatch(/class ReactDOMRoot \{[^}]*render\(children: number, \.\.\.args: readonly number\[\]\): number;[^}]*unmount\(\): void;/);
  expect(ts).toMatch(/class ReactDOMHydrationRoot \{[^}]*render\(children: number, \.\.\.args: readonly number\[\]\): number;/);
  type Root = { render(children: number, ...args: number[]): number; unmount(): void };
  const Root = lib.ReactDOMRoot as new (root: number) => Root;
  const Hydration = lib.ReactDOMHydrationRoot as new (root: number) => Root;
  const [a, b] = [new Root(10), new Hydration(20)];
  expect((lib.renderBoth as (a: Root, b: Root) => number)(a, b)).toBe(11 + 23);
  expect(a.render(1, 5, 6)).toBe(13);
  a.unmount();
  expect(() => a.render(1)).toThrow("Cannot update an unmounted root.");
  b.unmount();
  expect(b.render(1)).toBe(1);
});

// A trait of classes' method called while a guard is held runs what one of
// its classes' does, `extra`'s none, so the guard meets no borrow: its
// cell is its field, read plainly, not a field cell.
test("a trait of classes' method called under a guard runs what its classes' do", async () => {
  const { js, lib } = await compiled(
    "class-traits-under-a-guard",
    `use std::cell::RefCell;

pub trait Counted {
    fn cell(&self) -> &RefCell<Option<f64>>;

    fn extra(&self) -> f64 {
        1.0
    }

    fn total(&self) -> f64 {
        let values = self.cell().borrow();
        values.unwrap_or(0.0) + self.extra()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Tally {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub cell: RefCell<Option<f64>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Tally {
    pub fn new(first: f64) -> Self {
        Tally {
            cell: RefCell::new(Some(first)),
        }
    }
}

impl Counted for Tally {
    fn cell(&self) -> &RefCell<Option<f64>> {
        &self.cell
    }
}

pub fn total_of(tally: &Tally) -> f64 {
    tally.total()
}
`,
  );
  expect(js).not.toContain("$fieldCell");
  const Tally = lib.Tally as new (first: number) => unknown;
  expect((lib.totalOf as (tally: unknown) => number)(new Tally(2))).toBe(3);
});

test("a trait of classes bounds no type parameter and is no dyn's", () => {
  const generic = refused(
    "class-traits-generic",
    `${roots}
pub fn rendered<T: RootType>(root: &T) -> f64 {
    root.render(1.0, &[])
}
`,
  );
  expect(generic).toContain("implements `RootType`, a trait of classes, which bounds no type parameter");
  const dyn = refused(
    "class-traits-dyn",
    `${roots}
pub fn rendered(root: &ReactDOMRoot) -> f64 {
    let root: &dyn RootType = root;
    root.render(1.0, &[])
}
`,
  );
  expect(dyn).toContain("implements `RootType`, a trait of classes, of no `dyn`");
  const rest = refused(
    "class-traits-rest",
    roots.replace(
      "    fn unmount(&self) {\n        self.internal_root().replace(Some(0.0));",
      "    fn render(&self, children: f64, args: &[f64]) -> f64 {\n        children + args.len() as f64\n    }\n\n    fn unmount(&self) {\n        self.internal_root().replace(Some(0.0));",
    ),
  );
  expect(rest).toContain("takes a trait's method that takes rest arguments as the trait's default gives it");
  // Its method named as a field, which the instance's own property hides.
  const hidden = refused(
    "class-traits-hidden",
    roots.replaceAll("unmount", "count").replace(
      "pub struct ReactDOMRoot {\n",
      "pub struct ReactDOMRoot {\n    pub count: u32,\n",
    ).replace("ReactDOMRoot {\n            internal_root", "ReactDOMRoot {\n            count: 0,\n            internal_root"),
  );
  expect(hidden).toContain("has no method or constant named as its field, `count`");
  // A trait a struct that's no class implements too is a dictionary's,
  // which a class's impl is never.
  const mixed = refused(
    "class-traits-mixed",
    `${roots.replace("#[cfg_attr(rust_js, rust_js::rest)] ", "")}
pub struct Plain {
    pub internal_root: RefCell<Option<f64>>,
}

impl RootType for Plain {
    fn internal_root(&self) -> &RefCell<Option<f64>> {
        &self.internal_root
    }
}
`,
  );
  expect(mixed).toContain("implements no trait but a marker's, of no items or of types alone, not `RootType`");
  // The method that gives a field is no function to hold (rd304).
  const held = refused(
    "class-traits-accessor-held",
    `${roots}
pub fn roots(root: &ReactDOMRoot) -> bool {
    let given = <ReactDOMRoot as RootType>::internal_root;
    given(root).borrow().is_some()
}
`,
  );
  expect(held).toContain("method that gives its field, `internal_root`, is that field, read where it's called, `x.internal_root()`: no function to hold");
});
