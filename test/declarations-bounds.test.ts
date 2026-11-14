// A type parameter bound by a binding trait that says what a type it bounds
// is (#1866, ADR 0196 amended): webapi's `IsA<T>`, `#[rust_js::types =
// "T"]`, is TypeScript's constraint where the parameter's identity shows in
// the signature, in its result or written twice, `selected<T extends
// Element>(value: T): T`; erased to its type where it's one value's alone,
// as `children: C` of a `C: ReactNode` is, `label(value: Element)`. Codex's
// case (rust-js-lang/vercel-ui-sdk-in-rust): upstream's
// `composedClosest<T extends Element>`.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** TypeScript's errors of `file` in `dir`, strict, the DOM's lib in. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, `tsconfig-${file}.json`);
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", lib: ["es2022", "dom"], strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, types: [] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

// The trait by another name: its identity, not its spelling.
const program = `use webapi::{Element, IsA as Extends};

/// The same element it's given.
pub fn selected<T: Extends<Element>>(value: &T) -> &T {
    value
}

/// The first of those given, of their type.
pub fn first<T: Extends<Element>>(items: &[&'static T]) -> Option<&'static T> {
    items.first().copied()
}

/// Whether two are the same element, of one type: written twice.
pub fn same<T: Extends<Element>>(a: &T, b: &T) -> bool {
    std::ptr::eq(a, b)
}

/// Each given to \`f\`, of their type: in another parameter's bound.
pub fn each<T: Extends<Element>, F: Fn(&T)>(items: &[&T], f: F) {
    for item in items {
        f(item);
    }
}

/// How many are given, of any element: its type is one value's alone.
pub fn count<T: Extends<Element>>(items: &[&T]) -> usize {
    items.len()
}

/// Of an \`impl\`, one value's alone too.
pub fn held(items: &[&impl Extends<Element>]) -> bool {
    !items.is_empty()
}
`;

test("an IsA bound is TypeScript's constraint where its parameter's identity shows, else its type", () => {
  const dir = fixture("declarations-bounds");
  writeFileSync(join(dir, "lib.rs"), program);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function selected<T extends Element>(value: T): T;");
  expect(declarations).toContain("export function first<T extends Element>(items: T[]): T | null | undefined;");
  expect(declarations).toContain("export function same<T extends Element>(a: T, b: T): boolean;");
  expect(declarations).toContain("export function each<T extends Element>(items: T[], f: (t: T) => void): void;");
  expect(declarations).toContain("export function count(items: Element[]): number;");
  expect(declarations).toContain("export function held(items: Element[]): boolean;");
  // Codex's consumer: what Rust's bound refuses, TypeScript's does.
  writeFileSync(join(dir, "consumer.ts"), `import { count, each, first, held, same, selected } from "./lib.js";
const button = document.createElement("button");
export const kept: HTMLButtonElement = selected(button);
export const element: Element = selected<Element>(button);
export const picked: HTMLButtonElement | null | undefined = first([button]);
export const both: boolean = same(button, button);
each([button], (each: HTMLButtonElement) => each.disabled);
export const counted: number = count([button, document.body]);
export const any: boolean = held([button]);
// @ts-expect-error A string is no element.
selected<string>("button");
// @ts-expect-error A text node is no element.
selected<Text>(document.createTextNode("button"));
// @ts-expect-error Nor is it one to count.
count([document.createTextNode("button")]);
`);
  expect(typeChecked(dir, "consumer.ts")).toEqual([]);
});
