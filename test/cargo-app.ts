// A Cargo app of a test's Rust, a module of its own: in test/support.ts, which every test
// imports, its reads of react/ made each test a reader of them, so the
// selector ran the whole suite for a change to them (ADR 1101, amended).

import { dirname, join } from "node:path";
import { mkdirSync, writeFileSync } from "node:fs";

import { KeptApp, root } from "./support";

/** A Cargo package, `app`, its own workspace, in `dir`: `rust` at `lib`,
 * and React's crate by path where it uses it, as a host builds an app's
 * Rust (ADR 0101). `toml` is more of its `Cargo.toml`. What `checkCargo`
 * and a Cargo session are given of it. */
export function cargoApp(at: string | KeptApp, rust: string, { lib = "src/lib.rs", react = false, toml = "" } = {}) {
  // Into a fixture, or written through a kept app, each where it differs.
  const dir = typeof at === "string" ? at : at.dir;
  const write = (path: string, text: string) => {
    if (typeof at !== "string") return void at.write(path, text);
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), text);
  };
  write(lib, rust);
  const path = lib === "src/lib.rs" ? "" : `\n[lib]\npath = "${lib}"\n`;
  const dependencies = react ? `react = { package = "rust-js-react", path = ${JSON.stringify(join(root, "react"))} }\n` : "";
  write("Cargo.toml", `[package]\nname = "app"\nversion = "0.0.0"\nedition = "2024"\n${path}\n[dependencies]\n${dependencies}${toml ? `\n${toml}` : ""}\n[workspace]\n`);
  return { manifestPath: join(dir, "Cargo.toml"), packageNames: ["app"], offline: true };
}
