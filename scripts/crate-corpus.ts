// Which crates from crates.io rust-js compiles to JS (ROADMAP M8.2): each in a
// Cargo project of its own, every library of its graph compiled by rust-js
// as Cargo's `RUSTC_WRAPPER`, and what each refused first.
//
//   bun scripts/crate-corpus.ts [name ...]     the corpus, or the crates named
//
// Writes `target/crate-corpus.json` and prints a line per crate: its own
// verdict, then each crate of its graph rust-js refused, and why.

import { spawnSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { rustcShim } from "../tooling/cargo.js";

type Crate = { name: string; version: string; features?: string[]; defaultFeatures?: boolean };

// What shared models and the code around them commonly depend on.
const corpus: Crate[] = [
  { name: "either", version: "1.15.0" },
  { name: "itertools", version: "0.14.0" },
  { name: "indexmap", version: "2.11.4" },
  { name: "bitflags", version: "2.9.4" },
  { name: "smallvec", version: "1.15.1" },
  { name: "thiserror", version: "2.0.16" },
  { name: "anyhow", version: "1.0.99" },
  { name: "once_cell", version: "1.21.3" },
  { name: "semver", version: "1.0.27" },
  { name: "uuid", version: "1.18.1" },
  { name: "strum", version: "0.27.2", features: ["derive"] },
  { name: "url", version: "2.5.7" },
  { name: "regex", version: "1.11.3" },
  { name: "rust_decimal", version: "1.38.0" },
  { name: "chrono", version: "0.4.42", defaultFeatures: false, features: ["alloc"] },
  { name: "time", version: "0.3.44", defaultFeatures: false, features: ["alloc"] },
];

const root = resolve(import.meta.dir, "..");
const compiler = resolve(process.env.RUST_JS_COMPILER ?? join(root, "target/debug/rust-js"));
const toolchain = "1.98.1";
const wanted = process.argv.slice(2);
const chosen = wanted.length ? corpus.filter((c) => wanted.includes(c.name)) : corpus;

type Refusal = { crate: string; error: string };
type Result = { name: string; version: string; compiled: string[]; refused: Refusal[]; own: "compiles" | "refused" | "blocked" };
const results: Result[] = [];

for (const crate of chosen) {
  const dir = join(root, "target/crate-corpus", crate.name);
  mkdirSync(dir, { recursive: true });
  const spec = [
    `version = "${crate.version}"`,
    ...(crate.defaultFeatures === false ? ["default-features = false"] : []),
    ...(crate.features ? [`features = [${crate.features.map((f) => JSON.stringify(f)).join(", ")}]`] : []),
  ].join(", ");
  writeFileSync(
    join(dir, "Cargo.toml"),
    `[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "lib.rs"\n\n[dependencies]\n${crate.name} = { ${spec} }\n\n[workspace]\n`,
  );
  writeFileSync(join(dir, "lib.rs"), "");
  const run = spawnSync(
    "cargo",
    [`+${toolchain}`, "check", "--keep-going", "--message-format=json", "--target", "wasm32-unknown-unknown"],
    { cwd: dir, env: { ...process.env, RUSTC_WRAPPER: compiler, RUSTC: rustcShim(compiler) }, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" },
  );
  const compiled: string[] = [];
  const refused = new Map<string, string>();
  for (const line of run.stdout.split("\n").filter((l) => l.startsWith("{"))) {
    const message = JSON.parse(line);
    // `registry+https://..#name@1.2.3`, or `path+file://..#probe@0.0.0`.
    const id = String(message.package_id ?? "").split("#").pop() ?? "";
    if (message.reason === "compiler-artifact" && message.target?.kind?.some((k: string) => k === "lib" || k === "rlib")) {
      compiled.push(id);
    }
    if (message.reason === "compiler-message" && message.message?.level === "error" && !refused.has(id)) {
      const text = String(message.message.message);
      if (!text.startsWith("aborting due to")) refused.set(id, text);
    }
  }
  const own = (id: string) => id.startsWith(`${crate.name}@`);
  const result: Result = {
    name: crate.name,
    version: crate.version,
    compiled,
    refused: [...refused].map(([c, error]) => ({ crate: c, error })),
    own: compiled.some(own) ? "compiles" : refused.size && [...refused.keys()].some(own) ? "refused" : "blocked",
  };
  results.push(result);
  console.log(`${crate.name}@${crate.version}: ${result.own}, ${compiled.length} compiled, ${refused.size} refused`);
  for (const { crate: c, error } of result.refused) console.log(`  ${c}: ${error}`);
}

writeFileSync(join(root, "target/crate-corpus.json"), JSON.stringify(results, null, 2));
