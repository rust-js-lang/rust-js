// A binding's type a crate's nested module names by a path relative to the
// crate's root, `./math/color.js#Color`, is imported in its declarations
// from the declaring module's file, `../math/color.js`, as its JS imports
// it (ADR 0210, amended): TypeScript that imports the nested module checks.

import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";
import { typeChecked } from "./type-checked";

test("a relative binding type is imported from the declaring module's file", () => {
  buildReact();
  const dir = fixture("relative-declaration-binding");
  mkdirSync(join(dir, "math"));
  mkdirSync(join(dir, "render"));
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
  writeFileSync(join(dir, "lib.rs"), 'pub mod render;\n');
  writeFileSync(join(dir, "render.rs"), 'pub mod uniforms;\n');
  writeFileSync(join(dir, "math", "color.js"), 'export class Color { constructor(hex) { this.hex = hex; } }\n');
  writeFileSync(join(dir, "math", "color.d.ts"), 'export declare class Color { constructor(hex: number); hex: number; }\n');
  writeFileSync(join(dir, "render", "uniforms.rs"), `use std::marker::PhantomData;
use js::JsObject;

#[cfg_attr(rust_js, rust_js::types = "./math/color.js#Color")]
pub struct Color(PhantomData<JsObject>);

#[cfg_attr(rust_js, rust_js::link_name = "new ./math/color.js#Color")]
fn new_color(_hex: u32) -> &'static Color {
    unreachable!()
}

pub fn diffuse(hex: u32) -> &'static Color {
    new_color(hex)
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  expect(readFileSync(join(dir, "render", "uniforms.js"), "utf8")).toContain('import { Color } from "../math/color.js";');
  expect(readFileSync(join(dir, "render", "uniforms.d.ts"), "utf8")).toContain('import type { Color } from "../math/color.js";');
  const file = join(dir, "consumer.ts");
  writeFileSync(file, 'import { diffuse } from "./render/uniforms.js";\nconst hex: number = diffuse(1).hex;\n');
  expect(typeChecked(dir, file)).toEqual([]);
});
