// `@rust-js/std` (ADRs 0103, 0561, #1095), Rust's std for JS, the one
// package the JS rust-js writes imports, built the one way: `bun run std`
// writes, in `std/`, the helpers' module, `runtime.js`, `--runtime-module`'s;
// its types, `index.d.ts`, `--std-declarations`' (ADR 1149);
// std's bodies compiled, `iter.js`, its map and `manifest.json`; and its one
// entry, `index.js`, which re-exports both. `test/std-package.test.ts` builds
// it again elsewhere and compares.
//
//   bun scripts/std.ts [<root>]   # <root>/std/lib.rs into <root>/std/, the checkout's unless it's given
//
// Run in the root, the crate `std/lib.rs`, as its modules' first lines name
// it. Its manifest's crate hash is of its inputs alone, the same wherever and
// however it's built (`Manifest::of_its_inputs`, #1094).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

/** The command that compiles `<root>/std/lib.rs` into `<root>/std/` with
 * `compiler`, and where it's run, `<root>`. Its root module's JS, which
 * writes nothing, is `std.js`, not the package's entry. */
export function stdBuild(compiler: string, root: string): { cmd: string[]; cwd: string } {
  const cmd = [compiler, "std/lib.rs", "-o", "std/std.js", "--library", "--manifest", "std/manifest.json", "--std-bodies", "--", "--crate-name", "rust_js_std"];
  return { cmd, cwd: resolve(root) };
}

/** std built whole, `core`, `alloc` and `std`, of the toolchain's own
 * source, each body's MIR kept, for the target rust-js compiles for: the
 * sysroot the walk of where std raises its panics reads, `std`'s too
 * (#2292). Built once a toolchain by Cargo's `-Zbuild-std`, which keeps
 * it, in `target/std-raised`, of the source's vendored crates alone. */
export function wholeStd(root: string): { sysroot: string } | { error: string } {
  const dir = join(resolve(root), "target", "std-raised");
  const crate = join(dir, "crate");
  mkdirSync(join(crate, "src"), { recursive: true });
  writeFileSync(join(crate, "Cargo.toml"), '[package]\nname = "std-raised"\nversion = "0.0.0"\nedition = "2024"\n\n[workspace]\n');
  writeFileSync(join(crate, "src", "lib.rs"), "#![no_std]\n");
  const target = "wasm32-unknown-unknown";
  const built = spawnSync("cargo", ["build", "--quiet", "--release", "-Zbuild-std=std,panic_abort", `--target=${target}`, `--target-dir=${join(dir, "target")}`], {
    cwd: crate,
    encoding: "utf8",
    env: { ...process.env, RUSTC_BOOTSTRAP: "1", RUSTC_WRAPPER: "", RUSTFLAGS: "-Zalways-encode-mir" },
  });
  if (built.status !== 0) return { error: built.stderr };
  // Its crates, where Cargo builds each, as a sysroot's library.
  const lib = join(dir, "sysroot", "lib", "rustlib", target, "lib");
  rmSync(join(dir, "sysroot"), { recursive: true, force: true });
  mkdirSync(lib, { recursive: true });
  const rlibs = builtRlibs(join(dir, "target", target, "release"));
  if (typeof rlibs === "string") return { error: rlibs };
  for (const rlib of rlibs.values()) symlinkSync(rlib, join(lib, basename(rlib)));
  return { sysroot: join(dir, "sysroot") };
}

/** Each library Cargo built under `at`, by its file's name, `libstd_detect-
 * <hash>.rlib`: one of each, as a sysroot's library holds it, though Cargo
 * puts a build step's crate in its `out` and in `deps` both, CI's Cargo
 * the same file twice, which a second link of its name collided with
 * (`EEXIST`, run 45092210652). Of two of one name that differ, which is the
 * crate isn't known: said, never one chosen. */
export function builtRlibs(at: string): Map<string, string> | string {
  const found = new Map<string, string>();
  const walk = (dir: string): string | undefined => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        const differing = walk(path);
        if (differing) return differing;
        continue;
      }
      if (!entry.name.endsWith(".rlib") || entry.name === "libstd_raised.rlib") continue;
      const seen = found.get(entry.name);
      if (seen === undefined) found.set(entry.name, path);
      else if (!readFileSync(seen).equals(readFileSync(path))) return `two libraries of one name differ: ${seen} and ${path}`;
    }
    return undefined;
  };
  return walk(at) ?? found;
}

/** Where std raises the panics std's hook names its own lines of,
 * `--std-raised`'s walk of `wholeStd`'s MIR, of an empty crate: `Err`
 * naming one it can't find. */
export function raisedBuild(compiler: string, root: string, out: string): { code: number; stderr: string } {
  const whole = wholeStd(root);
  if ("error" in whole) return { code: 1, stderr: whole.error };
  const dir = mkdtempSync(join(tmpdir(), "rust-js-raised-"));
  try {
    writeFileSync(join(dir, "lib.rs"), "pub fn main() {}\n");
    const p = spawnSync(compiler, [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--std-raised", out, "--", "--edition=2024", `--sysroot=${whole.sysroot}`], { encoding: "utf8" });
    return { code: p.status ?? 1, stderr: p.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** Where the runtime keeps where std raises the panics std's hook names
 * its own lines of: a source of the compiler's, generated here (#2292). */
export const stdSitesFile = join(import.meta.dir, "..", "crates", "rust-js-program", "src", "runtime", "std_sites.js");

/** The runtime's `$stdSites` of `raised`, `--std-raised`'s: each site under
 * the toolchain's `/rustc/<commit>/`, as its shipped std names its files,
 * `rustc -vV`'s commit, on every machine (ADR 0262's exception, #2292). */
export function stdSites(raised: string, root: string): string {
  const version = spawnSync("rustc", ["-vV"], { cwd: resolve(root), encoding: "utf8" }).stdout;
  const commit = /^commit-hash: ([0-9a-f]{40})$/m.exec(version)?.[1];
  if (commit === undefined) throw new Error(`rustc -vV names no commit:\n${version}`);
  const placed = (sites: unknown): unknown =>
    Array.isArray(sites)
      ? sites.map((site) => vendoredSite(site, root) ?? `/rustc/${commit}/${site}`)
      : Object.fromEntries(Object.entries(sites as object).map(([tag, each]) => [tag, placed(each)]));
  // And the root itself, `source`, which a std location the compiler
  // writes in a program's JS is joined to: a shim's of std's, or a std
  // function's own made a `fn` pointer (#2335).
  const text = JSON.stringify({ source: `/rustc/${commit}`, ...(placed(JSON.parse(raised)) as object) }, null, 2);
  return `// Where std raises each panic std's hook names its own line of, \`file:line:column\`
// under the toolchain's \`/rustc/<commit>/\`, as its shipped std names its
// files, each a constant of the toolchain, whatever calls it; of a site that
// differs by type, each type's (#2292, ADR 0262's exception).
// Generated by \`bun run std\` from std's MIR (\`compiled_std/raised.rs\`). Do not edit.
const $stdSites = ${text};
`;
}

/** A site in a crate std's source vendors, `library/vendor/hashbrown-0.17.1/
 * src/raw.rs:1289:21`, as the toolchain's shipped std names it: under the
 * registry its own build took the crate from, which the shipped rlib of
 * that crate, for the target rust-js compiles for, records of its files,
 * `/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1/src/raw.rs`, a
 * constant of the toolchain (#2325). `undefined` of a site of std's own. */
function vendoredSite(site: string, root: string): string | undefined {
  const at = /^library\/vendor\/(([^/]+)-[0-9][^/]*)\/(.*?):(\d+):(\d+)$/.exec(site);
  if (at === null) return undefined;
  const [, dir, crate, file, line, column] = at;
  const sysroot = spawnSync("rustc", ["--print", "sysroot"], { cwd: resolve(root), encoding: "utf8" }).stdout.trim();
  const lib = join(sysroot, "lib", "rustlib", "wasm32-unknown-unknown", "lib");
  const named = new RegExp(`(/[\\x21-\\x7e]*?/)${dir.replaceAll(".", "\\.")}/${file.replaceAll(".", "\\.")}`);
  for (const rlib of readdirSync(lib).filter((name) => name.startsWith(`lib${crate.replaceAll("-", "_")}-`) && name.endsWith(".rlib"))) {
    const found = named.exec(readFileSync(join(lib, rlib), "latin1"));
    if (found !== null) return `${found[1]}${dir}/${file}:${line}:${column}`;
  }
  throw new Error(`the shipped ${crate} names no ${dir}/${file}, which std's site ${site} is in`);
}

/** The catalogue of std's bodies `@rust-js/std` compiles, as `compiler`
 * walks std's MIR, written to `out` (#2305): of an empty crate compiled as
 * every crate is, of the toolchain's std, whose crates' hashes it keeps,
 * not std's own build's, of a `core` built to unwind. */
export function catalogueBuild(compiler: string, out: string): { code: number; stderr: string } {
  const dir = mkdtempSync(join(tmpdir(), "rust-js-catalogue-"));
  try {
    writeFileSync(join(dir, "lib.rs"), "pub fn main() {}\n");
    const p = spawnSync(compiler, [join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--std-catalogue", out, "--", "--edition=2024"], { encoding: "utf8" });
    return { code: p.status ?? 1, stderr: p.stderr };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** The package's entry, of its manifest's modules: the helpers and each
 * module of std's bodies, re-exported. */
export function stdEntry(manifest: { modules: { file: string }[] }): string {
  const modules = ["runtime.js", ...manifest.modules.map((module) => module.file)];
  return `// @rust-js/std: the helpers and std's bodies the JS rust-js writes imports, its one entry.\n// Generated by \`bun run std\`. Do not edit.\n${modules.map((file) => `export * from "./${file}";\n`).join("")}`;
}

/** `text` as `path`, written only where it differs from what's there: a
 * check that makes std/ again, `std-fresh.ts`, a push's, leaves a fresh
 * one's files as they were, written at no new time, which a push reads as
 * written while it was checked (#2415), as the compiler leaves its own. */
export function writeChanged(path: string, text: string) {
  if (existsSync(path) && readFileSync(path, "utf8") === text) return;
  writeFileSync(path, text);
}

/** The whole package in `<root>/std/` with `compiler`: its helpers, std's
 * bodies, its entry, and the catalogue of them every compile reads. */
export function buildStd(compiler: string, root: string): { code: number; stderr: string } {
  const std = join(resolve(root), "std");
  const helpers = spawnSync(compiler, ["--runtime-module"], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  if (helpers.status !== 0) return { code: helpers.status ?? 1, stderr: helpers.stderr };
  writeChanged(join(std, "runtime.js"), helpers.stdout);
  // Its types, what a crate's declarations name of std (ADR 1149).
  const declarations = spawnSync(compiler, ["--std-declarations"], { encoding: "utf8" });
  if (declarations.status !== 0) return { code: declarations.status ?? 1, stderr: declarations.stderr };
  writeChanged(join(std, "index.d.ts"), declarations.stdout);
  const scratch = mkdtempSync(join(tmpdir(), "rust-js-raised-"));
  try {
    const raised = join(scratch, "raised.json");
    const recorded = raisedBuild(compiler, root, raised);
    if (recorded.code !== 0) return recorded;
    const { cmd, cwd } = stdBuild(compiler, root);
    const bodies = spawnSync(cmd[0], cmd.slice(1), { cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
    if (bodies.status !== 0) return { code: bodies.status ?? 1, stderr: bodies.stderr };
    writeChanged(join(std, "index.js"), stdEntry(JSON.parse(readFileSync(join(std, "manifest.json"), "utf8"))));
    // What every compile reads of the catalogue, where it walked std's MIR.
    const built = catalogueBuild(compiler, join(std, "compiled-std.json"));
    if (built.code !== 0) return built;
    // The runtime's sites, a source of the compiler's: where they moved, a
    // new toolchain's, written, and the compiler and std built again of
    // them, here alone; elsewhere they're the checkout's to make.
    const sites = stdSites(readFileSync(raised, "utf8"), root);
    if (sites === readFileSync(stdSitesFile, "utf8")) return built;
    if (resolve(root) !== resolve(join(import.meta.dir, ".."))) return { code: 1, stderr: `${stdSitesFile} isn't std's sites: run \`bun run std\`` };
    writeFileSync(stdSitesFile, sites);
    const rebuilt = spawnSync("cargo", ["build", "--quiet"], { cwd: resolve(root), encoding: "utf8" });
    if (rebuilt.status !== 0) return { code: rebuilt.status ?? 1, stderr: rebuilt.stderr };
    return buildStd(compiler, root);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

if (import.meta.main) {
  const checkout = join(import.meta.dir, "..");
  const built = buildStd(join(checkout, "target", "debug", "rust-js"), process.argv[2] ?? checkout);
  if (built.code !== 0) console.error(built.stderr);
  process.exit(built.code);
}
