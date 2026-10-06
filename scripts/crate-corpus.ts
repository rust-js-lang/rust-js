// Which crates from crates.io rust-js compiles to JS (ROADMAP M8.2): each in a
// Cargo project of its own, every library of its graph compiled by rust-js
// as Cargo's `RUSTC_WRAPPER`, and what each refused first. A crate with a
// probe, `crate-corpus/<name>.rs`, a library of the project that uses it and
// has a `report()`, is run too where it compiles: its report, the JS's and
// native Rust's, the same or not. Compiled isn't run: bitflags' JS compiled
// and didn't parse.
//
//   bun scripts/crate-corpus.ts [name ...]     the corpus, or the crates named
//
// Writes `target/crate-corpus.json` and prints a line per crate: its own
// verdict, then each crate of its graph rust-js refused, and why, then
// whether its probe runs as natively.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
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
const toolchain = "1.99.0";
const wanted = process.argv.slice(2);
const chosen = wanted.length ? corpus.filter((c) => wanted.includes(c.name)) : corpus;

type Refusal = { crate: string; error: string };
type Result = {
  name: string;
  version: string;
  compiled: string[];
  refused: Refusal[];
  own: "compiles" | "refused" | "blocked";
  // What Cargo said last where nothing was refused, though the crate wasn't
  // compiled: a crash, or a build script's failure.
  unexplained?: string;
  // Its probe's report, the JS's beside native Rust's: `same`, or where
  // they differ, or why the probe didn't compile or run.
  runs?: string;
};

/** The first line where the JS's report isn't native Rust's. */
function difference(native: string, js: string): string {
  const [a, b] = [native.split("\n"), js.split("\n")];
  const at = a.findIndex((line, i) => line !== b[i]);
  const line = at === -1 ? a.length : at;
  return `line ${line + 1}: native ${JSON.stringify(a[line] ?? "")}, JS ${JSON.stringify(b[line] ?? "")}`;
}
const results: Result[] = [];

for (const crate of chosen) {
  const dir = join(root, "target/crate-corpus", crate.name);
  mkdirSync(dir, { recursive: true });
  const spec = [
    `version = "${crate.version}"`,
    ...(crate.defaultFeatures === false ? ["default-features = false"] : []),
    ...(crate.features ? [`features = [${crate.features.map((f) => JSON.stringify(f)).join(", ")}]`] : []),
  ].join(", ");
  const probe = join(import.meta.dir, "crate-corpus", `${crate.name}.rs`);
  writeFileSync(
    join(dir, "Cargo.toml"),
    `[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "lib.rs"\n\n[[bin]]\nname = "native"\npath = "main.rs"\n\n[dependencies]\n${crate.name} = { ${spec} }\n\n[workspace]\n`,
  );
  writeFileSync(join(dir, "lib.rs"), existsSync(probe) ? readFileSync(probe, "utf8") : "");
  writeFileSync(join(dir, "main.rs"), "fn main() {\n    print!(\"{}\", probe::report());\n}\n");
  const run = spawnSync(
    "cargo",
    [`+${toolchain}`, "check", "--lib", "--keep-going", "--message-format=json", "--target", "wasm32-unknown-unknown"],
    { cwd: dir, env: { ...process.env, RUSTC_WRAPPER: compiler, RUSTC: rustcShim(compiler) }, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" },
  );
  const compiled: string[] = [];
  const refused = new Map<string, string>();
  // The probe's JS: its manifest is the first line of the `.rust-js` beside
  // its metadata, and its crate root's JS beside that.
  let probeJs: string | undefined;
  for (const line of run.stdout.split("\n").filter((l) => l.startsWith("{"))) {
    const message = JSON.parse(line);
    // `registry+https://..#name@1.2.3`, or `path+file://..#probe@0.0.0`.
    const id = String(message.package_id ?? "").split("#").pop() ?? "";
    if (message.reason === "compiler-artifact" && message.target?.kind?.some((k: string) => k === "lib" || k === "rlib")) {
      compiled.push(id);
      const marker = message.filenames?.find((f: string) => f.endsWith(".rmeta"))?.replace(/\.rmeta$/, ".rust-js");
      if (id.startsWith("probe@") && marker && existsSync(marker)) {
        probeJs = join(dirname(readFileSync(marker, "utf8").split("\n")[0]), "lib.js");
      }
    }
    if (message.reason === "compiler-message" && String(message.message?.level).startsWith("error") && !refused.has(id)) {
      const text = String(message.message.message);
      if (!text.startsWith("aborting due to")) refused.set(id, text);
    }
  }
  const own = (id: string) => id.startsWith(`${crate.name}@`);
  // The probe is the corpus's, not the crate's graph.
  const probeRefused = [...refused].find(([id]) => id.startsWith("probe@"))?.[1];
  const result: Result = {
    name: crate.name,
    version: crate.version,
    compiled: compiled.filter((id) => !id.startsWith("probe@")),
    refused: [...refused].filter(([id]) => !id.startsWith("probe@")).map(([c, error]) => ({ crate: c, error })),
    own: compiled.some(own) ? "compiles" : refused.size && [...refused.keys()].some(own) ? "refused" : "blocked",
  };
  if (result.own !== "compiles" && !result.refused.length) {
    result.unexplained = run.stderr.trim().split("\n").slice(-12).join("\n");
  }
  if (result.own === "compiles" && existsSync(probe)) {
    if (probeRefused !== undefined) result.runs = `the probe is refused: ${probeRefused}`;
    else if (!probeJs) result.runs = "the probe wasn't compiled";
    else {
      // Native Rust's report, built for the host, apart from rust-js's.
      const native = spawnSync("cargo", [`+${toolchain}`, "run", "--quiet", "--bin", "native", "--target-dir", join(dir, "native")], {
        cwd: dir,
        maxBuffer: 64 * 1024 * 1024,
        encoding: "utf8",
      });
      const js = spawnSync(process.execPath, ["-e", `import { report } from ${JSON.stringify(probeJs)};\nprocess.stdout.write(report());`], {
        cwd: dir,
        maxBuffer: 64 * 1024 * 1024,
        encoding: "utf8",
      });
      if (native.status !== 0) result.runs = `native Rust failed: ${native.stderr.trim().split("\n").slice(-3).join(" ")}`;
      else if (js.status !== 0) result.runs = `the JS failed: ${js.stderr.trim().split("\n").slice(-6).join(" ")}`;
      else result.runs = native.stdout === js.stdout ? "same" : `differs at ${difference(native.stdout, js.stdout)}`;
    }
  }
  results.push(result);
  console.log(`${crate.name}@${crate.version}: ${result.own}, ${result.compiled.length} compiled, ${result.refused.length} refused`);
  for (const { crate: c, error } of result.refused) console.log(`  ${c}: ${error}`);
  if (result.unexplained) console.log(result.unexplained.replace(/^/gm, "  | "));
  if (result.runs) console.log(`  runs: ${result.runs}`);
}

writeFileSync(join(root, "target/crate-corpus.json"), JSON.stringify(results, null, 2));
