// Which crates from crates.io rust-js compiles to JS (ROADMAP M8.2): each in a
// Cargo project of its own, every library of its graph compiled by rust-js
// as Cargo's `RUSTC_WRAPPER`, and what each refused first. A crate with a
// probe, `crate-corpus/<name>.rs`, a library of the project that uses it and
// has a `report()`, is run too where it compiles: its report, the JS's and
// native Rust's, the same or not. Compiled isn't run: bitflags' JS compiled
// and didn't parse.
//
//   bun scripts/crate-corpus.ts [name ...]     the corpus, or the crates named
//   bun scripts/crate-corpus.ts --check [name ...]
//                                              and fail where one does worse
//                                              than docs/crate-corpus.md says
//
// The crates, each at its exact release, are docs/crate-corpus.md's rows.
// Writes `target/crate-corpus.json` and prints a line per crate: its own
// verdict, then each crate of its graph rust-js refused, and why, then
// whether its probe runs as natively.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { cargoEnv } from "../tooling/cargo.js";

type Verdict = "runs" | "compiles" | "refused" | "blocked";

/** A crate of the corpus, as its row in docs/crate-corpus.md says: its exact
 * release; the features it's built with, its default ones off, where the
 * row names any; and the verdict that's its bar, with what an open issue
 * says it does now, where one does. */
export type Crate = { name: string; version: string; features?: string[]; bar: Verdict; open?: { now: Verdict; issue: string } };

/** docs/crate-corpus.md's table, the corpus's one list (ADR 1067): each row
 * whose second cell is a verdict is a crate, its first cell the crate's
 * name, its exact release and, in parentheses, its features, `chrono 0.4.45
 * (`alloc`)`. A row of no exact release is refused: crates.io's newest
 * release had turned crates red with no change here (either 1.19.0,
 * smallvec 1.16.4), and a new release is measured by editing its row. The
 * verdict is `runs`, or `runs (refused: #467)` where an open issue says
 * the crate does worse now. Or why the table can't be read. */
export function corpusRows(doc: string): Crate[] | { problem: string } {
  const crates: Crate[] = [];
  for (const line of doc.split("\n")) {
    const cells = line.split("|").map((cell) => cell.trim());
    const said = cells[2]?.match(/^(runs|compiles|refused|blocked)(?: \((runs|compiles|refused|blocked): (#\d+)\))?$/);
    if (cells.length <= 3 || !said) continue;
    const named = cells[1].match(/^([\w-]+) (\d+\.\d+\.\d+)(?: \((`[\w-]+`(?:, `[\w-]+`)*)\))?$/);
    if (!named) {
      return {
        problem: `docs/crate-corpus.md's row \`${cells[1]}\` names no exact release: write the crate's name and version, \`either 1.19.0\`, and its features after them, \`chrono 0.4.45 (\`alloc\`)\``,
      };
    }
    const [, bar, now, issue] = said as [string, Verdict, Verdict | undefined, string | undefined];
    crates.push({
      name: named[1],
      version: named[2],
      ...(named[3] ? { features: named[3].split(", ").map((feature) => feature.slice(1, -1)) } : {}),
      bar,
      ...(now && issue ? { open: { now, issue } } : {}),
    });
  }
  return crates;
}

/** The dependency on `crate` its Cargo project writes: its release exactly,
 * and only its row's features, where it names any. */
export function dependency(crate: Crate): string {
  const spec = [
    `version = "=${crate.version}"`,
    ...(crate.features ? ["default-features = false", `features = [${crate.features.map((f) => JSON.stringify(f)).join(", ")}]`] : []),
  ].join(", ");
  return `${crate.name} = { ${spec} }`;
}

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

function main() {
  const root = resolve(import.meta.dir, "..");
  const compiler = resolve(process.env.RUST_JS_COMPILER ?? join(root, "target/debug/rust-js"));
  const toolchain = "1.99.0";
  const corpus = corpusRows(readFileSync(join(root, "docs/crate-corpus.md"), "utf8"));
  if (!Array.isArray(corpus)) {
    console.error(corpus.problem);
    process.exit(1);
  }
  const check = process.argv.includes("--check");
  const wanted = process.argv.slice(2).filter((arg) => arg !== "--check");
  const unknown = wanted.filter((name) => !corpus.some((c) => c.name === name));
  if (unknown.length) {
    console.error(`docs/crate-corpus.md has no row of ${unknown.join(", ")}`);
    process.exit(1);
  }
  const chosen = wanted.length ? corpus.filter((c) => wanted.includes(c.name)) : corpus;
  const results: Result[] = [];

  for (const crate of chosen) {
    const dir = join(root, "target/crate-corpus", crate.name);
    mkdirSync(dir, { recursive: true });
    const probe = join(import.meta.dir, "crate-corpus", `${crate.name}.rs`);
    writeFileSync(
      join(dir, "Cargo.toml"),
      `[package]\nname = "probe"\nversion = "0.0.0"\nedition = "2024"\n\n[lib]\npath = "lib.rs"\n\n[[bin]]\nname = "native"\npath = "main.rs"\n\n[dependencies]\n${dependency(crate)}\n\n[workspace]\n`,
    );
    writeFileSync(join(dir, "lib.rs"), existsSync(probe) ? readFileSync(probe, "utf8") : "");
    writeFileSync(join(dir, "main.rs"), "fn main() {\n    print!(\"{}\", probe::report());\n}\n");
    // Every crate the probe builds is rust-js's to compile, a registry's too.
    // Each a library, as a crate of crates.io is to the apps that use it.
    const metadata = spawnSync("cargo", [`+${toolchain}`, "metadata", "--format-version=1"], { cwd: dir, maxBuffer: 256 * 1024 * 1024, encoding: "utf8" });
    const crates = (JSON.parse(metadata.stdout).packages as { manifest_path: string }[]).map((pkg) => realpathSync(dirname(pkg.manifest_path)));
    const run = spawnSync(
      "cargo",
      [`+${toolchain}`, "check", "--lib", "--keep-going", "--message-format=json", "--target", "wasm32-unknown-unknown"],
      { cwd: dir, env: cargoEnv(compiler, undefined, { apps: [], crates }), maxBuffer: 256 * 1024 * 1024, encoding: "utf8" },
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

  // `--check`, the nightly's: what docs/crate-corpus.md records of each
  // crate is the bar, a crate doing worse failing the run, as chrono and
  // rust_decimal went from runs to refused unseen (#467, #468). A row
  // marked with an open issue, `runs (refused: #467)`, holds while the
  // crate does just that, and fails once it does better, so the mark goes
  // with the issue.
  if (check) {
    const rank: Record<Verdict, number> = { runs: 3, compiles: 2, refused: 1, blocked: 1 };
    const verdict = (r: Result): Verdict => (r.own === "compiles" && r.runs === "same" ? "runs" : r.own);
    const failed: string[] = [];
    for (const r of results) {
      const row = chosen.find((c) => c.name === r.name)!;
      const got = verdict(r);
      if (row.open && got === row.open.now) console.log(`${r.name}: ${got}, as ${row.open.issue} says, where it ${row.bar}`);
      else if (row.open && rank[got] > rank[row.open.now]) failed.push(`${r.name}: ${got}, better than ${row.open.issue} says: write \`${got}\` in its row`);
      else if (rank[got] < rank[row.open?.now ?? row.bar]) failed.push(`${r.name}: ${got}, where docs/crate-corpus.md says ${row.open?.now ?? row.bar}`);
    }
    for (const line of failed) console.error(line);
    if (failed.length) process.exit(1);
  }
}

if (import.meta.main) main();
