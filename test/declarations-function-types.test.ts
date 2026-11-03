// What a crate's declarations (ADR 0196) say of a function type's
// parameters: each named as the Rust names it, a fn pointer's as its type
// writes it and a field's `dyn Fn` as every literal of the crate fills it
// (#543), and its trailing `Option`s optional, as a function's are, since a
// caller may leave them out (ADR 0313, #544).

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

const program = `use std::rc::Rc;

pub struct Node {
    pub id: u32,
}

pub struct System {
    pub link: Rc<dyn Fn(&Node, &Node, u32) -> u32>,
    pub unlink: Rc<dyn Fn(u32, Option<&Node>) -> u32>,
    pub propagate: Box<dyn Fn(u32, bool) -> u32>,
    pub named: fn(count: u32, _sum: u32) -> u32,
    pub unnamed: fn(u32) -> u32,
    pub given: Rc<dyn Fn(u32) -> u32>,
    pub shown: Option<Box<dyn Fn(&Node) -> String>>,
    pub ignored: Box<dyn Fn(u32)>,
    pub pair: Box<dyn Fn(&Node, &Node) -> u32>,
}

fn total(count: u32, total: u32) -> u32 {
    count + total
}

fn doubled(n: u32) -> u32 {
    n * 2
}

fn describe(item: &Node) -> String {
    format!("node {}", item.id)
}

pub fn system(given: Rc<dyn Fn(u32) -> u32>, inner: bool) -> System {
    let base = Rc::new(7);
    System {
        link: {
            let base = Rc::clone(&base);
            Rc::new(move |dep: &Node, sub: &Node, version| dep.id + sub.id + version + *base)
        },
        unlink: Rc::new(|link, sub| link + sub.map_or(0, |sub| sub.id)),
        propagate: Box::new(move |link, inner_write| if inner_write || inner { link } else { 0 }),
        named: total,
        unnamed: doubled,
        given,
        shown: Some(Box::new(describe)),
        ignored: Box::new(|_| {}),
        pair: Box::new(|a, _| a.id),
    }
}

pub fn other() -> System {
    System {
        link: Rc::new(|dep: &Node, target: &Node, _| dep.id + target.id),
        unlink: Rc::new(|link, _sub| link),
        propagate: Box::new(|link, _| link),
        named: total,
        unnamed: |x| x,
        given: Rc::new(doubled),
        shown: None,
        ignored: Box::new(|_| {}),
        pair: Box::new(|b, _| b.id),
    }
}

pub fn calls(f: impl Fn(u32, Option<u32>) -> u32, g: impl Fn(Option<u32>, u32) -> u32) -> u32 {
    f(1, None) + g(None, 2)
}
`;

test("a function type's parameters are named as the Rust names them, and its trailing Options may be left out", () => {
  const dir = fixture("declarations-function-types");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  // Each literal names `dep` first; one names `sub` second, another `target`,
  // and one `_` third: those are named by their types.
  expect(declarations).toContain("link: (dep: Node, node: Node, value: number) => number;");
  // `_sub` is `sub`, as a function's parameter is (ADR 0678), and optional.
  expect(declarations).toContain("unlink: (link: number, sub?: Node | null | undefined) => number;");
  expect(declarations).toContain("propagate: (link: number, value: boolean) => number;");
  // A fn pointer's, as its type writes them, over its literals' `total(count,
  // total)`, `_sum` as `sum`; one that names none as its
  // literals fill it, which here name it apart, by its type.
  expect(declarations).toContain("named: (count: number, sum: number) => number;");
  expect(declarations).toContain("unnamed: (value: number) => number;");
  // Filled with what names none, a parameter of `system`, in one literal.
  expect(declarations).toContain("given: (value: number) => number;");
  // A function, `describe(item)`, in one literal; `None` in the other names
  // none, so it's named by its type.
  expect(declarations).toContain("shown?: ((node: Node) => string) | undefined;");
  // A `_`, which each literal writes, names none.
  expect(declarations).toContain("ignored: (value: number) => void;");
  // Two its literals name apart, named by their types, each its own.
  expect(declarations).toContain("pair: (node: Node, node2: Node) => number;");
  // An `impl Fn`'s trailing `Option`, which a call leaves out, `f(1)`; not
  // one before a parameter that isn't.
  expect(declarations).toContain(
    "export function calls(f: (value: number, value2?: number | null | undefined) => number, g: (value: number | null | undefined, value2: number) => number): number;",
  );
  // TypeScript calls them leaving the trailing `Option` out, and gives a
  // callback that takes fewer parameters.
  writeFileSync(
    join(dir, "use.ts"),
    `import { calls, other, system } from "./lib.js";
const s = system((n) => n + 1, true);
export const linked: number = s.link({ id: 1 }, { id: 2 }, 3);
export const unlinked: number = s.unlink(4) + s.unlink(4, { id: 5 });
export const both: number = calls((a) => a, (b, c) => (b ?? 0) + c);
export const wrong: number = other().unlink();
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, exactOptionalPropertyTypes: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.ts(6,");
});

// A struct literal in a variant, and one read through a block: the names
// of what fills each.
test("a variant's function field is named as its literals fill it", () => {
  const dir = fixture("declarations-function-types-variant");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    `pub enum Kind {
    Computed { getter: Box<dyn Fn(Option<u32>) -> u32> },
    Plain,
}

pub fn computed(start: u32) -> Kind {
    Kind::Computed {
        getter: {
            let start = start + 1;
            Box::new(move |previous_value| previous_value.unwrap_or(start))
        },
    }
}

pub fn plain() -> Kind {
    Kind::Plain
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("getter: (previousValue?: number | null | undefined) => number;");
});

// A JS class's constructor is JS's, given each `None` as `undefined` (ADR
// 0330): none of its parameters is optional. Its fn pointer's names are
// its parameters'.
test("a class's constructor names its parameters as its type writes them, none optional", () => {
  buildReact();
  const dir = fixture("declarations-function-types-class");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    `use js::{Class, Unknown};

pub fn made(ctor: &'static Class<fn(name: &'static str, size: Option<f64>) -> Unknown>) -> &'static Unknown {
    js::new(ctor, ("a", None))
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function made(ctor: new (name: string, size: number | null | undefined) => unknown): unknown;");
});
