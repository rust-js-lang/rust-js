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
  // Filled with what names none, a parameter of `system`, in one literal,
  // which says nothing of them, and `doubled(n)` in the other (#630).
  expect(declarations).toContain("given: (n: number) => number;");
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

// A function type's parameter is named after its type's last word, and one
// JS keeps is named apart: a `ResolveFunction`'s `function$`, as React's
// Flight Node loader's hooks are given, where `function` made a `.d.ts`
// TypeScript can't parse (rd673).
test("declarations name a function type's parameter apart from a word JS keeps", () => {
  const dir = fixture("declarations-kept-parameter");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    "#[repr(transparent)]\n#[derive(Clone, Copy)]\npub struct ResolveFunction(pub &'static dyn Fn(&str, ResolveFunction) -> u32);\n\npub fn resolve(next: ResolveFunction) -> u32 {\n    (next.0)(\"a\", next)\n}\n",
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).toContain("function$: ResolveFunction");
  expect(readFileSync(join(dir, "lib.d.ts"), "utf8")).not.toContain("function:");
});

// A static's or a constant's `dyn Fn` or fn pointer is named as the
// functions its value may be name them, a thread-local's by its `init`, of
// each branch that gives one, a host's `js::cast` saying nothing; and an
// alias written in it, inside a private alias's `dyn Fn(PriorityLevel, ..)`
// too, by its name: SchedulerNative's `unstable_scheduleCallback` (#630).
test("a static's function type is named as the functions its value may be", () => {
  buildReact();
  const dir = fixture("declarations-value-function-names");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/value_function_names.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { PriorityLevel } from "./priorities.js";');
  expect(declarations).toContain(
    "export const SCHEDULE_CALLBACK: (priorityLevel: PriorityLevel, label: string, delay?: number | null | undefined) => Task;",
  );
  expect(declarations).toContain("export const getCurrentPriorityLevel: () => PriorityLevel;");
  expect(declarations).toContain("export const SCALE: (value: number, factor: number) => number;");
  // A `pub` alias of a lifetime the function's `'_` gives, by its name.
  expect(declarations).toContain("export function visit<T>(value: T, f: Visit<T>): boolean;");
  writeFileSync(
    join(dir, "use.ts"),
    `import { getCurrentPriorityLevel, SCALE, SCHEDULE_CALLBACK } from "./lib.js";
import type { PriorityLevel } from "./priorities.js";

const level: PriorityLevel = getCurrentPriorityLevel();
const task = SCHEDULE_CALLBACK(level, "work");
console.log(JSON.stringify([task.id, task.priorityLevel, SCHEDULE_CALLBACK(1, "a", 2).id, SCALE(2, 3)]));
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  expect(checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"))).toEqual([]);
  // No host installs one here: this crate's, as Rust's.
  expect(JSON.parse(run([process.execPath, join(dir, "use.ts")]))).toEqual([4, 3, 3, 6]);
});
