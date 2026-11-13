// What a crate's declarations (ADR 0196) say of a type with no values:
// `!`, and so `Infallible`, which Rust 1.100 made `!`, is TypeScript's
// `never` wherever it's written (ADR 0386, amended), so an
// `Option<Infallible>` is declared as the `null` or `undefined` it can only
// be, as React's `IntentionallyUnusedArgument?: empty` writes it.

import { expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compiler, fixture, root, run } from "./support";

test("an Option of Infallible is declared as only null or undefined", () => {
  const dir = fixture("declarations-infallible");
  writeFileSync(join(dir, "Cargo.toml"), '[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n');
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::convert::Infallible;

pub fn set(priority: u32, _unused: Option<Infallible>) -> u32 {
    priority
}

pub struct Halted {
    pub value: Option<Infallible>,
}

pub fn absurd(v: Infallible) -> u32 {
    match v {}
}
`,
  );
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js")]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("export function set(priority: number, unused?: never | null | undefined): number;");
  expect(declarations).toContain("    value?: never | undefined;");
  expect(declarations).toContain("export function absurd(v: never): number;");
  // A caller may leave it out or give `null`, and nothing else.
  writeFileSync(
    join(dir, "use.ts"),
    'import { set, type Halted } from "./lib.js";\nexport const a: number = set(1) + set(2, null) + set(3, undefined);\nexport const h: Halted = {};\n// @ts-expect-error: no value is an `Infallible`\nset(4, 0);\n',
  );
  writeFileSync(
    join(dir, "tsconfig.json"),
    JSON.stringify({ compilerOptions: { module: "esnext", moduleResolution: "bundler", allowJs: true, strict: true, noEmit: true, skipLibCheck: false }, files: ["use.ts"] }),
  );
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", join(dir, "tsconfig.json")], { cwd: dir });
  expect(checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"))).toEqual([]);
});
