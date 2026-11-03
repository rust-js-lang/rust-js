// What a crate's declarations (ADR 0196) say of a function whose JS takes,
// after its parameters, what a JS caller can't give (#549, ADR 1077): a
// fact of a type parameter, `size(TSize)`, or a bound's dictionary,
// `shown(value, TDisplay)`. It isn't declared: a note is where it would be,
// so TypeScript that imports it is refused, and a Rust caller, which gives
// each, calls it as before.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

const program = `use std::any::{Any, TypeId};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Display;

pub fn size<T>() -> usize {
    size_of::<T>()
}

pub fn alignment<T>() -> usize {
    align_of::<T>()
}

pub fn named<T>() -> &'static str {
    std::any::type_name::<T>()
}

pub use self::named as unstable_named;

pub fn is_u32<T: 'static>(_value: T) -> bool {
    TypeId::of::<T>() == TypeId::of::<u32>()
}

pub fn boxed<T: 'static>(value: T) -> Box<dyn Any> {
    Box::new(value)
}

pub fn drops<T>() -> bool {
    std::mem::needs_drop::<T>()
}

pub fn counted<T: std::hash::Hash + Eq>(items: Vec<T>) -> usize {
    let mut counts: HashMap<T, u32> = HashMap::new();
    for item in items {
        *counts.entry(item).or_default() += 1;
    }
    counts.len()
}

pub fn distinct<T: std::hash::Hash + Eq>(items: Vec<T>) -> usize {
    items.into_iter().collect::<HashSet<T>>().len()
}

pub fn ordered<T: Ord>(items: Vec<T>) -> usize {
    items.into_iter().collect::<BTreeSet<T>>().len()
}

pub fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort_unstable();
    items
}

pub fn shown<T: Display>(value: T) -> String {
    format!("<{value}>")
}

pub fn first<T: Clone>(items: &[T]) -> Option<T> {
    items.first().cloned()
}

pub struct Counter {
    pub count: u32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { count: 0 }
    }

    pub fn size_of<T>(&self) -> usize {
        size_of::<T>()
    }
}

pub fn total() -> usize {
    size::<u64>() + named::<u8>().len() + usize::from(is_u32(1u32)) + Counter::new().size_of::<u16>()
}
`;

test("a function whose JS takes a fact of a type or a dictionary is a note, not declared", () => {
  const dir = fixture("declarations-ungiven");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Its JS is as before: a Rust caller gives each.
  expect(js).toContain("export function size(TSize) {");
  expect(js).toContain("export function shown(value, TDisplay) {");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const note = (name: string, what: string) =>
    `// \`${name}\` isn't declared: its JS takes ${what} after its arguments, which a JS caller can't give (ADR 1077).`;
  expect(declarations).toContain(note("size", "`T`'s size_of"));
  expect(declarations).toContain(note("alignment", "`T`'s align_of"));
  expect(declarations).toContain(note("named", "`T`'s type_name"));
  expect(declarations).toContain(note("isU32", "`T`'s the `TypeId`"));
  expect(declarations).toContain(note("boxed", "`T`'s the `TypeId`"));
  expect(declarations).toContain(note("drops", "`T`'s needs_drop"));
  expect(declarations).toContain(note("counted", "`T`'s a map's class"));
  expect(declarations).toContain(note("distinct", "`T`'s a map's class"));
  expect(declarations).toContain(note("ordered", "`T`'s a map's class"));
  expect(declarations).toContain(note("sorted", "`T`'s an unstable sort"));
  expect(declarations).toContain(note("shown", "`T`'s `Display` dictionary"));
  for (const name of ["size", "alignment", "named", "isU32", "boxed", "drops", "counted", "distinct", "ordered", "sorted", "shown"]) {
    expect(declarations).not.toContain(`export function ${name}`);
  }
  // Its export by another name too (ADR 0596).
  expect(declarations).not.toContain("unstable_named");
  // A method whose JS takes one is left out of its object; a `T: Clone`,
  // whose clone of a value JS leaves out, is declared.
  expect(declarations).not.toContain("sizeOf<");
  expect(declarations).toContain(note("Counter.sizeOf", "`T`'s size_of"));
  expect(declarations).toContain("export function first<T>(items: T[]): T | null | undefined;");
  expect(declarations).toContain("export function total(): number;");
  // TypeScript that imports one is refused; one declared is called.
  writeFileSync(
    join(dir, "use.ts"),
    `import { first, total } from "./lib.js";
import { size } from "./lib.js";
export const ok: number = total() + (first([1]) ?? 0);
export const wrong = size();
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
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.length).toBe(1);
  expect(errors[0]).toContain("use.ts(2,");
  expect(errors[0]).toContain("has no exported member 'size'");
});

// A module namespace's function (ADR 1037), an object of methods' (ADR
// 0943) and a class's method and static (ADR 0761) whose JS takes one are
// left out too, each a note after what it's of (#559).
test("a namespace's, an object's and a class's member whose JS takes a fact is a note", () => {
  buildReact();
  const dir = fixture("declarations-ungiven-members");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/ungiven_members.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("Widget.prototype.isU32 = function (TId) {");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const note = (name: string, what: string) =>
    `// \`${name}\` isn't declared: its JS takes ${what} after its arguments, which a JS caller can't give (ADR 1077).`;
  expect(declarations).toContain("export function count(n: number): number;");
  expect(declarations).not.toContain("export function sizeOf");
  expect(declarations).toContain(note("sizeOf", "`T`'s size_of"));
  expect(declarations).not.toContain("named<T>");
  expect(declarations).toContain(note("Scope.named", "`T`'s type_name"));
  expect(declarations).not.toContain("isU32<");
  expect(declarations).not.toContain("static shown");
  expect(declarations).toContain("    area(): number;");
  // Each class's note after it, before what follows.
  const isU32 = declarations.indexOf(note("Widget.isU32", "`T`'s the `TypeId`"));
  const shown = declarations.indexOf(note("Widget.shown", "`T`'s `Display` dictionary"));
  expect(isU32).toBeGreaterThan(declarations.indexOf("export declare class Widget"));
  expect(shown).toBeGreaterThan(isU32);
  expect(declarations.indexOf("export function total")).toBeGreaterThan(shown);
});
