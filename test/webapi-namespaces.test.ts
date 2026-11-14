// A WebIDL namespace is a module of the webapi crate, its functions and its
// constants (#1779): every namespace @webref/idl declares whose members
// TypeScript's DOM has, so a namespace isn't left out as WebGPU's flags,
// `GPUBufferUsage` and the rest, were. A constant is a Rust one of its
// WebIDL type, read as its JS name, as an interface's is.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { idlNamespaceConstants, typescriptNamespaces } from "../webapi/coverage";
import { buildReact, compiler, fixture, root, run, target } from "./support";
import { snake } from "../scripts/rust-names";

beforeAll(buildReact, 600_000);

test("every namespace's constants TypeScript's DOM has are its module's", async () => {
  const crate = readFileSync(join(root, "webapi/src/lib.rs"), "utf8");
  const declared = await typescriptNamespaces();
  const missing: string[] = [];
  let constants = 0;
  for (const [namespace, names] of await idlNamespaceConstants()) {
    for (const name of names.filter((name) => declared.get(namespace)?.has(name))) {
      constants++;
      const bound = `#[cfg_attr(rust_js, rust_js::link_name = "${namespace}.${name}")]\n    pub const ${name}:`;
      if (!crate.includes(`pub mod ${snake(namespace)} {`) || !crate.includes(bound)) missing.push(`${namespace}.${name}`);
    }
  }
  expect(constants).toBeGreaterThanOrEqual(26);
  expect(missing).toEqual([]);
});

test("WebGPU's flags are each its namespace's constants", async () => {
  const dir = fixture("webapi-namespaces");
  writeFileSync(join(dir, "lib.rs"), `use webapi::{gpu_buffer_usage, gpu_color_write, gpu_map_mode, gpu_shader_stage, gpu_texture_usage};

pub fn usage() -> u32 {
    gpu_buffer_usage::UNIFORM | gpu_buffer_usage::COPY_DST
}

pub fn flags() -> Vec<u32> {
    vec![
        gpu_texture_usage::RENDER_ATTACHMENT,
        gpu_map_mode::READ,
        gpu_shader_stage::FRAGMENT,
        gpu_color_write::ALL,
    ]
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // A `u32`'s `|`, of globals whose values JS reads as it runs, kept a
  // `u32` (ADR 0011).
  expect(js).toContain("return (GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST) >>> 0;");
  // A browser's globals, as WebGPU's spec gives them.
  const globals = globalThis as Record<string, unknown>;
  Object.assign(globals, {
    GPUBufferUsage: { UNIFORM: 0x40, COPY_DST: 0x08 },
    GPUTextureUsage: { RENDER_ATTACHMENT: 0x10 },
    GPUMapMode: { READ: 0x1 },
    GPUShaderStage: { FRAGMENT: 0x2 },
    GPUColorWrite: { ALL: 0xf },
  });
  const lib = await import(join(dir, "lib.js"));
  expect([lib.usage(), lib.flags()]).toEqual([0x48, [0x10, 0x1, 0x2, 0xf]]);
});
