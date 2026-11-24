// What compat.test.ts and compat-surfaces.test.ts share (#2465): an app
// built by `rust-js build`, and the git crates that stand in for
// crates.io's, beside rust-js's own wasm-bindgen, js-sys and web-sys
// (ADR 1112).
import { expect } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { compiler, root } from "./support";

export const cli = join(root, "tooling", "cli.js");
export const build = (dir: string) => runSync([process.execPath, cli, "build"], dir, 300_000, { RUST_JS_COMPILER: compiler });
export const git = (dir: string, ...args: string[]) => runSync(["git", "-c", "user.name=t", "-c", "user.email=t@t", ...args], dir, 60_000);

/** A crate `name` in a git repository of its own, at `dir/name`, whose
 * dependencies are `deps` and root `source`, committed: by default a
 * stand-in for uuid 1, which rust-js's crates serve
 * (`tooling/compat/covered.txt`). Not getrandom's: a getrandom that isn't
 * crates.io's is refused, rust-js writing its API itself (#2326). */
export function gitCrate(dir: string, deps: string, source: string, name = "uuid", version = "1.18.1") {
  const crate = join(dir, name);
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), `[package]\nname = "${name}"\nversion = "${version}"\nedition = "2024"\n\n[dependencies]\n${deps}`);
  writeFileSync(join(crate, "src", "lib.rs"), source);
  if (!existsSync(join(crate, ".git"))) expect(git(crate, "init", "-q").code).toBe(0);
  for (const args of [["add", "."], ["commit", "-q", "-m", name]]) expect(git(crate, ...args).code).toBe(0);
  return crate;
}

/** An app at `dir/app` of no npm packages that depends on each git crate
 * of `crates`, its root `source`: the binding crates rust-js's own use are
 * the checkout's. */
export function app(dir: string, crates: string | string[], source = "pub fn answer() -> u32 {\n    uuid::answer()\n}\n") {
  const at = join(dir, "app");
  mkdirSync(join(at, "src"), { recursive: true });
  const deps = [crates].flat().map((crate) => `${crate.split("/").at(-1)} = { git = ${JSON.stringify(`file://${crate}`)} }`).join("\n");
  writeFileSync(join(at, "Cargo.toml"), `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\n${deps}\n\n[workspace]\n`);
  writeFileSync(join(at, "src", "lib.rs"), source);
  return at;
}

export const config = (at: string) => readFileSync(join(at, ".cargo", "config.toml"), "utf8");
export const usesWasmBindgen = 'wasm-bindgen = "0.2.98"\njs-sys = "0.3"\n';
export const answer = "use js_sys as _;\nuse wasm_bindgen as _;\n\npub fn answer() -> u32 {\n    42\n}\n";
