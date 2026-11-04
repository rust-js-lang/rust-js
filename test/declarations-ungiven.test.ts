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
  // A trait of classes is its classes' prototypes' methods, no dictionary
  // of it declared (rd283, #550).
  expect(declarations).toContain("    sides(): number;");
  expect(declarations).not.toContain("Sided<");
  expect(js).not.toContain("widgetSided");
  expect(declarations).not.toContain("widgetSided");
  // Each class's note after it, before what follows.
  const isU32 = declarations.indexOf(note("Widget.isU32", "`T`'s the `TypeId`"));
  const shown = declarations.indexOf(note("Widget.shown", "`T`'s `Display` dictionary"));
  expect(isU32).toBeGreaterThan(declarations.indexOf("export declare class Widget"));
  expect(shown).toBeGreaterThan(isU32);
  expect(declarations.indexOf("export function total")).toBeGreaterThan(shown);
});

// A crate's `pub` trait is the type of its dictionaries, each dictionary
// of an impl its module exports a `const` of it, and a bound's dictionary
// of it a function takes is declared, so the function is too (#550, ADR
// 1077, amended). TypeScript calls it with a crate's dictionary or one of
// its own, and gets what Rust would.
test("a trait is the type of its dictionaries, which a function's declaration takes", () => {
  const dir = fixture("declarations-dictionaries");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/declared_dictionaries.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const shape = readFileSync(join(dir, "shape.d.ts"), "utf8");
  expect(shape).toContain(`export interface Shape<Self> {
    /** Its area. */
    area(self: Self): number;
    describe(self: Self): string;
}`);
  expect(shape).toContain("export const circleShape: Shape<Circle>;");
  expect(shape).toContain("export const rectShape: Shape<Rect>;");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain('import type { Shape } from "./shape.js";');
  expect(declarations).toContain(`export interface Named<Self> {
    make(name: string): Self;
    name(self: Self): string;
}`);
  expect(declarations).toContain("export const tagNamed: Named<Tag>;");
  expect(declarations).toContain("export const tagShape: Shape<Tag>;");
  expect(declarations).toContain("export function totalArea<S>(shapes: S[], SShape: Shape<S>): number;");
  // A trailing `Option` before a dictionary is given, `null` for `None`.
  expect(declarations).toContain(
    "export function described<S>(shape: S, prefix: string | null | undefined, SShape: Shape<S>): string;",
  );
  expect(declarations).toContain(
    "export function renamed<N>(named: N, to: string, NNamed: Named<N>, NShape: Shape<N>): string;",
  );
  // A trait of a `&mut self` method is, its `self` the box its entry takes
  // (#662), and a function of it.
  expect(declarations).toContain("export interface Counter<Self> {\n    bump(self: {\n        value: Self;\n    }): number;\n}");
  expect(declarations).toContain("export function bumped<C>(c: {\n    value: C;\n}, CCounter: Counter<C>): number;");
  // A trait whose dictionary holds what else isn't declared, nor is a
  // function of it; a generic impl's dictionary isn't.
  const note = (name: string, what: string) =>
    `// \`${name}\` isn't declared: its JS takes ${what} after its arguments, which a JS caller can't give (ADR 1077).`;
  expect(declarations).toContain(note("sides", "`S`'s `Sided` dictionary"));
  expect(declarations).toContain(note("labels", "`L`'s `Labeled` dictionary"));
  expect(declarations).toContain(note("checked", "`C`'s `Checked` dictionary"));
  expect(declarations).toContain(note("shows", "`S`'s `Shows` dictionary"));
  expect(declarations).toContain(note("names", "`N`'s `Names` dictionary"));
  expect(declarations).toContain(note("shown", "`T`'s `Display` dictionary"));
  for (const name of ["Sided", "Labeled", "Checked", "Shows", "Names", "wrapperShape"]) {
    expect(declarations).not.toContain(name + "<");
    expect(declarations).not.toContain(`export const ${name}`);
  }
  writeFileSync(
    join(dir, "use.ts"),
    `import { circleShape, rectShape, type Shape } from "./shape.js";
import { described, renamed, tagNamed, tagShape, totalArea } from "./lib.js";

interface Square {
  side: number;
}
const squareShape: Shape<Square> = { area: (s) => s.side * s.side, describe: (s) => \`a square of side \${s.side}\` };
console.log(
  JSON.stringify([
    totalArea([{ radius: 1 }, { radius: 2 }], circleShape),
    totalArea([{ width: 2, height: 3 }], rectShape),
    totalArea([{ side: 3 }], squareShape),
    described({ width: 1, height: 2 }, "it's ", rectShape),
    described({ radius: 1 }, null, circleShape),
    described({ side: 2 }, undefined, squareShape),
    renamed({ text: "a" }, "b", tagNamed, tagShape),
  ]),
);
`,
  );
  writeFileSync(
    join(dir, "wrong.ts"),
    `import { circleShape, rectShape } from "./shape.js";
import { totalArea } from "./lib.js";
import { bumped } from "./lib.js";
import { wrapperShape } from "./lib.js";
export const mixed = totalArea([{ radius: 1 }], rectShape);
export const missing = totalArea([{ radius: 1 }]);
export const ok = totalArea([{ radius: 1 }], circleShape);
export const unboxed = bumped(1, { bump: () => 1 });
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts", "wrong.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.filter((line) => line.startsWith("use.ts"))).toEqual([]);
  expect(errors.map((line) => line.replace(/: error.*/, ""))).toEqual(["wrong.ts(4,10)", "wrong.ts(5,35)", "wrong.ts(6,24)", "wrong.ts(8,31)"]);
  // What it gives is what Rust's calls give.
  const ran = run([process.execPath, join(dir, "use.ts")]);
  expect(JSON.parse(ran)).toEqual([
    15.707963267948966,
    6,
    9,
    "it's a 1x2 rectangle",
    "a shape of area 3.141592653589793",
    "a square of side 2",
    "a b 0",
  ]);
});

// A `#[track_caller]` function's caller's location (ADR 0427) and a const
// parameter's value (ADR 0107), which its JS takes after its arguments, are
// what a JS caller gives as a Rust caller does: declared, in their order
// before its dictionaries, neither optional (#560, ADR 1077, amended).
test("a function's caller's location and a const parameter's value are declared", () => {
  const dir = fixture("declarations-evidence-given");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/evidence_given.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain("export function checked(n, why, location) {");
  expect(js).toContain("export function sized(shapes, location, N, SShape) {");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const location = "{\n    readonly file: string;\n    readonly line: number;\n    readonly column: number;\n}";
  expect(declarations).toContain(`export function here(location: ${location}): ${location};`);
  expect(declarations).toContain(`export function checked(n: number, why: string | null | undefined, location: ${location}): number;`);
  // One that reads no location takes none, its `Option` left out as before.
  expect(declarations).toContain("export function unwrapped(n?: number | null | undefined): number;");
  expect(declarations).toContain("export function zeros(N: number): number[];");
  expect(declarations).toContain("export function big(N: bigint): bigint;");
  expect(declarations).toContain(`export function sized<S>(shapes: S[], location: ${location}, N: number, SShape: Shape<S>): string;`);
  expect(declarations).toContain("    width(grid: Grid, W: number): number;");
  expect(declarations).not.toContain("isn't declared");
  writeFileSync(
    join(dir, "use.ts"),
    `import { big, checked, Grid, here, sized, zeros } from "./lib.js";
import { squareShape } from "./shape.js";

const at = { file: "use.ts", line: 4, column: 12 };
let panicked = "";
try {
  checked(0, null, at);
} catch (error) {
  panicked = (error as Error).message;
}
console.log(
  JSON.stringify([here(at), checked(2, undefined, at), zeros(3), String(big(4n)), sized([{ side: 2 }], at, 1, squareShape), panicked, Grid.width({ cells: [] }, 4)]),
);
`,
  );
  writeFileSync(
    join(dir, "wrong.ts"),
    `import { checked, zeros } from "./lib.js";
export const unplaced = checked(1, null);
export const unsized = zeros();
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts", "wrong.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.map((line) => line.replace(/: error.*/, ""))).toEqual(["wrong.ts(2,25)", "wrong.ts(3,24)"]);
  // What it gives is what Rust's calls give, a panic naming where it's told it's called from.
  expect(JSON.parse(run([process.execPath, join(dir, "use.ts")]))).toEqual([
    { file: "use.ts", line: 4, column: 12 },
    2,
    [0, 0, 0],
    "4",
    "1 of area 4 at 4",
    "zero at use.ts:4:12",
    4,
  ]);
});

// A `&mut` its JS has as a box, or a handle, whose `value` it reads and
// writes, is declared as that, `bump(n: { value: number })`, and one to an
// object as the object (ADRs 0074, 0099); a `dyn` of a declared trait as
// its pair, `{ value: unknown; impl: Shape<unknown> }` (ADR 0049), of
// another trait `any`, not a function (#619, #620, ADR 1077, amended).
test("a `&mut` that's a box is declared as its box, and a `dyn` as its pair", () => {
  const dir = fixture("declarations-boxes");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/declared_boxes.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // Its JS is as before.
  expect(js).toContain("n.value = (n.value + 1) >>> 0;");
  expect(js).toContain("return { value: { side }, impl: squareShape };");
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const held = (ty: string, indent = "") => `{\n${indent}    value: ${ty};\n${indent}}`;
  expect(declarations).toContain(`export function bump(n: ${held("number")}): void;`);
  expect(declarations).toContain(`export function set<T>(slot: ${held("T")}, value: T): void;`);
  expect(declarations).toContain(`export function shout(text: ${held("string")}): void;`);
  expect(declarations).toContain(`export function fill(slot: ${held("number | null | undefined")}): void;`);
  // A `&mut` it gives back is a handle on the item.
  expect(declarations).toContain(`export function largest(items: number[]): ${held("number")};`);
  // A closure's `&mut` parameter, as generic code gives it; a `&mut` to a
  // closure is the closure, of the function its bound says (#661).
  expect(declarations).toContain(`export function apply(f: (value: ${held("number")}) => void, n: ${held("number")}): void;`);
  expect(declarations).toContain("export function call(f: () => void): void;");
  expect(declarations).toContain(`    raise(level: ${held("Level", "    ")}): void;`);
  // A `&mut` to an object is the object.
  expect(declarations).toContain("export function tick(counter: Counter): void;");
  // A method's own type parameter's bound is its own: a `&mut F` of an
  // `F: FnMut(u32)` is the function.
  expect(declarations).toContain("    each(counter: Counter, f: (value: number) => void): void;");
  expect(declarations).toContain("export function grow(items: number[]): void;");
  const pair = (indent = "") => `{\n${indent}    value: unknown;\n${indent}    impl: Shape<unknown>;\n${indent}}`;
  expect(declarations).toContain('import type { Shape } from "./shape.js";');
  expect(declarations).toContain(`export function square(side: number): ${pair()};`);
  expect(declarations).toContain(`export function areaOf(shape: ${pair()}): number;`);
  expect(declarations).toContain(`export function total(shapes: ${pair()}[]): number;`);
  // Of a trait of a `&mut self` method, declared now (#662), its pair too.
  expect(declarations).toContain("export const counterSteps: Steps<Counter>;");
  expect(declarations).toContain(`export function stepper(): {\n    value: unknown;\n    impl: Steps<unknown>;\n};`);
  // Of one that gives a `&mut`, declared now (#679), its pair; of a trait
  // that isn't declared, one of an associated type, `any`.
  expect(declarations).toContain(`export function slots(): {\n    value: unknown;\n    impl: Slots<unknown>;\n};`);
  expect(declarations).toContain("export function firsts(): any;");
  expect(declarations).not.toContain("Firsts<");
  expect(declarations).not.toContain("any[]");
  writeFileSync(
    join(dir, "use.ts"),
    `import { squareShape } from "./shape.js";
import { apply, areaOf, bump, call, Counter, fill, grow, largest, Level, set, shout, slots, square, tick, total } from "./lib.js";

const n = { value: 1 };
bump(n);
const slot = { value: "a" };
set(slot, "b");
const text = { value: "hi" };
shout(text);
const filled: { value: number | null | undefined } = { value: null };
fill(filled);
fill(filled);
const items = [3, 9, 4];
largest(items).value = 0;
const m = { value: 5 };
apply((x) => {
  x.value *= 2;
}, m);
let calls = 0;
call(() => {
  calls += 1;
});
const level: { value: Level } = { value: "Low" };
Level.raise(level);
const counter = { count: 0 };
tick(counter);
let seen = 0;
Counter.each(counter, (count: number) => {
  seen = count;
});
const grown: number[] = [];
grow(grown);
grow(grown);
const made = square(3);
const held = slots();
held.impl.slot(held).value += 3;
console.log(
  JSON.stringify([
    n.value,
    slot.value,
    text.value,
    filled.value,
    items,
    m.value,
    calls,
    level.value,
    counter.count,
    seen,
    grown,
    areaOf(made),
    made.impl.area(made.value),
    areaOf({ value: { side: 2 }, impl: squareShape }),
    total([square(1), { value: { side: 2 }, impl: squareShape }]),
    (held.value as Counter).count,
  ]),
);
`,
  );
  // What the declarations before said, each refused; and a box where an
  // object is, refused too.
  writeFileSync(
    join(dir, "wrong.ts"),
    `import { areaOf, bump, grow, largest, Level, set, shout, square, tick } from "./lib.js";
bump(1);
set(1, 2);
shout("x");
Level.raise("Low");
export const next = largest([1]) + 1;
export const called = square(1)();
export const area = areaOf(() => 1);
tick({ value: { count: 0 } });
grow({ value: [] });
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts", "wrong.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.filter((line) => line.startsWith("use.ts"))).toEqual([]);
  expect(errors.map((line) => line.replace(/: error.*/, ""))).toEqual([
    "wrong.ts(2,6)",
    "wrong.ts(3,5)",
    "wrong.ts(4,7)",
    "wrong.ts(5,13)",
    "wrong.ts(6,21)",
    "wrong.ts(7,23)",
    "wrong.ts(8,28)",
    "wrong.ts(9,8)",
    "wrong.ts(10,8)",
  ]);
  // What it gives is what Rust's calls give: `2 b hi! Some(2) [3, 0, 4] 10 1
  // High 1 1 [0, 1] 9 9 4 5 3`, the same calls' natively.
  expect(JSON.parse(run([process.execPath, join(dir, "use.ts")]))).toEqual([
    2,
    "b",
    "hi!",
    2,
    [3, 0, 4],
    10,
    1,
    "High",
    1,
    1,
    [0, 1],
    9,
    9,
    4,
    5,
    3,
  ]);
});

// A trait whose methods take `&mut self` is declared as the type of its
// dictionaries too (#662): its `&mut self` is the box its entry takes, `{
// value: Self }`, as `MutRefForm::ts_type` says in its trait's terms (ADR
// 1086), and so is a function's `&mut C` it's given with its dictionary,
// a `&mut Self` besides its receiver and a `&mut u32` it gives (#679), and
// one within an `Option` it takes (#686). One with a `&mut Self` elsewhere
// within what it takes, `Vec<&mut Self>`, isn't (#682).
test("a trait of `&mut self` methods is the type of its dictionaries", () => {
  const dir = fixture("declarations-mut-traits");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), readFileSync(join(import.meta.dir, "fixtures/declared_mut_traits.rs"), "utf8"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  const held = (ty: string) => `{\n        value: ${ty};\n    }`;
  expect(declarations).toContain(`export interface Counter<Self> {
    bump(self: ${held("Self")}): void;
    add(self: Self, total: ${held("number")}): void;
    fill(self: Self, out: number[]): void;
    get(self: Self): number;
    reset(self: ${held("Self")}, to: number): number;
}`);
  expect(declarations).toContain("export const clicksCounter: Counter<Clicks>;");
  expect(declarations).toContain("export const u32Counter: Counter<number>;");
  expect(declarations).toContain("export function bumpTwice<C>(c: {\n    value: C;\n}, CCounter: Counter<C>): number;");
  expect(declarations).toContain("export function bumpDyn(c: {\n    value: unknown;\n    impl: Counter<unknown>;\n}): number;");
  // A `&mut Self` besides its receiver is the box its entry takes too, and a
  // `&mut u32` it gives a box, `{ value }` (#679).
  expect(declarations).toContain(`export interface Swap<Self> {
    swapWith(self: ${held("Self")}, other: ${held("Self")}): void;
}`);
  expect(declarations).toContain("export const u32Swap: Swap<number>;");
  expect(declarations).toContain("export const clicksSwap: Swap<Clicks>;");
  expect(declarations).toContain(`export interface Slot<Self> {
    slot(self: ${held("Self")}): ${held("number")};
}`);
  expect(declarations).toContain("export const clicksSlot: Slot<Clicks>;");
  expect(declarations).toContain("export const u32Slot: Slot<number>;");
  expect(declarations).toContain(`    keep(self: ${held("Self")}, into: ${held("Self | null | undefined")}): boolean;`);
  // One within an `Option`, generic code's box, which each entry adapts (#686).
  expect(declarations).toContain(`export interface Pick<Self> {
    pick(self: ${held("Self")}, other?: ${held("Self")} | null | undefined): boolean;
}`);
  // One elsewhere within what it takes, which no entry makes its impl's own (#682).
  expect(declarations).not.toContain("Gather");
  writeFileSync(
    join(dir, "use.ts"),
    `import { bumpDyn, bumpTwice, type Clicks, clicks, clicksCounter, clicksKeep, clicksSlot, clicksSwap, type Counter, u32Counter, u32Keep, u32Slot, u32Swap } from "./lib.js";
const c = { value: clicks() };
const twice = bumpTwice(c, clicksCounter);
const n = { value: 5 };
const fromNumber = bumpTwice(n, u32Counter);
const total = { value: 0 };
clicksCounter.add(c.value, total);
const out: number[] = [];
u32Counter.fill(7, out);
const old = clicksCounter.reset(c, 3);
const viaDyn = bumpDyn({ value: c.value, impl: clicksCounter });
const counters: Counter<number>[] = [u32Counter];
const x = { value: 3 };
const y = { value: 4 };
u32Swap.swapWith(x, y);
const p = { value: clicks() };
const q = { value: { n: 9 } };
clicksSwap.swapWith(p, q);
clicksSlot.slot(p).value += 1;
const u = { value: 5 };
u32Slot.slot(u).value += 2;
const kept: { value: Clicks | null | undefined } = { value: null };
const hadClicks = clicksKeep.keep(p, kept);
const keptNumber: { value: number | null | undefined } = { value: 3 };
const hadNumber = u32Keep.keep(u, keptNumber);
console.log(JSON.stringify([twice, fromNumber, n.value, total.value, out, old, c.value.n, viaDyn, counters.length, x.value, y.value, p.value.n, q.value.n, u.value, hadClicks, kept.value?.n, hadNumber, keptNumber.value]));
`,
  );
  // A value where its box is, refused, and a box where a value is.
  writeFileSync(
    join(dir, "wrong.ts"),
    `import { bumpTwice, clicks, clicksCounter, clicksSlot, u32Swap } from "./lib.js";
bumpTwice(clicks(), clicksCounter);
clicksCounter.bump(clicks());
u32Swap.swapWith({ value: 3 }, 4);
const n: number = clicksSlot.slot({ value: clicks() });
`,
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: { module: "esnext", moduleResolution: "bundler", strict: true, noEmit: true, skipLibCheck: false, allowJs: true, typeRoots: [join(root, "node_modules/@types")] },
      files: ["use.ts", "wrong.ts"],
    }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  const errors = checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
  expect(errors.filter((line) => line.startsWith("use.ts"))).toEqual([]);
  expect(errors.map((line) => line.replace(/: error.*/, ""))).toEqual(["wrong.ts(2,11)", "wrong.ts(3,20)", "wrong.ts(4,32)", "wrong.ts(5,7)"]);
  // What Rust's calls give: `bump_twice` of a `Clicks` 2, of a `5u32` 7, its
  // `add` 2, `fill` `[7]`, `reset` the 2 it had, then 5, and `bump_dyn` 6;
  // `swap_with` of `3u32` and `4u32` 4 and 3, of `Clicks`' `0` and `9` 9
  // and 0, whose `*slot() += 1` makes 10, and `5u32`'s `*slot() += 2` 7;
  // `keep` of the `Clicks` into `None` false, `Some(Clicks { n: 11 })`, and
  // of the `7u32` into `Some(3)` true, `Some(8)`.
  expect(JSON.parse(run([process.execPath, join(dir, "use.ts")]))).toEqual([2, 7, 7, 2, [7], 2, 6, 6, 1, 4, 3, 10, 0, 7, false, 11, true, 8]);
});
