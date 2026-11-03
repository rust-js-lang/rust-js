// An inherent impl's constants (#498): the `pub` ones another crate or JS
// can name are properties of their type's object, `export const
// ReactiveFlags = { NONE: 0, .. }`, declared so, and each use is still the
// value where it's written, as TypeScript's `const enum` is (ADR 0410).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

function compile(name: string, source: string): { js: string; dts: string } {
  const dir = fixture(name);
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(join(dir, "lib.rs"), source);
  const js = ["--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...js]);
  return { js: readFileSync(join(dir, "lib.js"), "utf8"), dts: readFileSync(join(dir, "lib.d.ts"), "utf8") };
}

const flags = `pub struct ReactiveFlags;

impl ReactiveFlags {
    pub const NONE: i32 = 0;
    pub const MUTABLE: i32 = 1;
    #[cfg_attr(rust_js, rust_js::name = "RecursedCheck")]
    pub const RECURSED_CHECK: i32 = 4;
    const PRIVATE: i32 = 8;
}

pub fn watched(flags: i32) -> bool {
    flags & (ReactiveFlags::MUTABLE | ReactiveFlags::RECURSED_CHECK | ReactiveFlags::PRIVATE) != ReactiveFlags::NONE
}
`;

test("a type's pub constants are its object's, each use its value", () => {
  const { js, dts } = compile("impl-constants-flags", flags);
  expect(js).toContain("export const ReactiveFlags = { NONE: 0, MUTABLE: 1, RecursedCheck: 4 };");
  expect(js).not.toContain("PRIVATE");
  expect(js).toContain("return (flags & (1 | 4 | 8)) !== 0;");
  expect(dts).toContain(
    "export const ReactiveFlags: {\n    readonly NONE: number;\n    readonly MUTABLE: number;\n    readonly RecursedCheck: number;\n};",
  );
});

test("a private type's constants, and a generic impl's, are no object", () => {
  const { js, dts } = compile(
    "impl-constants-private",
    "struct Limits;\n\nimpl Limits {\n    pub const MAX: u32 = 9;\n}\n\npub struct Wrapped<T>(pub T);\n\nimpl<T> Wrapped<T> {\n    pub const ONE: u32 = 1;\n}\n\npub fn max() -> u32 {\n    Limits::MAX + Wrapped::<u8>::ONE\n}\n",
  );
  expect(js).not.toContain("Limits");
  expect(js).not.toContain("Wrapped");
  expect(js).toContain("return 10;");
  expect(dts).not.toContain("Limits");
  expect(dts).not.toContain("export const Wrapped");
});

test("a type's object of constants and methods isn't declared in part", () => {
  const { js, dts } = compile(
    "impl-constants-methods",
    "pub struct Counter {\n    pub count: u32,\n}\n\nimpl Counter {\n    pub const START: u32 = 0;\n\n    pub fn new() -> Counter {\n        Counter { count: Counter::START }\n    }\n}\n",
  );
  expect(js).toContain("export const Counter = {\n  START: 0,\n\n  new() {\n    return { count: 0 };\n  },\n};");
  expect(dts).not.toContain("export const Counter");
});

test("a class's constant is its prototype's, no object's", () => {
  const { js, dts } = compile(
    "impl-constants-class",
    "#[cfg_attr(rust_js, rust_js::class)]\npub struct Component {\n    pub props: f64,\n}\n\nunsafe impl js::Defined for Component {}\nimpl js::Classed for Component {\n    type Constructor = js::AnyConstructor;\n}\n\n#[cfg_attr(rust_js, rust_js::class)]\nimpl Component {\n    pub const LIMIT: f64 = 1.0;\n\n    fn new(props: f64) -> Self {\n        Component { props }\n    }\n}\n\npub fn made(props: f64) -> f64 {\n    Component::new(props).props + Component::LIMIT\n}\n",
  );
  expect(js).toContain("Component.prototype.limit = 1;");
  expect(dts).not.toContain("export const Component");
});
