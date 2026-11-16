// Pack the crates rust-js releases on npm, `@rust-js/builtins`,
// `@rust-js/webapi`, `@rust-js/react`, `@rust-js/next`, `@rust-js/node` and `@rust-js/react-native`: each is Cargo's own packaging of its crate, which
// `package-crates.ts` makes and verifies with a plain stable rustc, in an
// npm package that names the crate. What a crate shares with an app, the
// crates it depends on, is a peer dependency, so an app has one copy of
// each; Cargo finds them in its `node_modules` by a patch. This never
// publishes.
//
//   bun scripts/package-npm-crates.ts <out-dir>   # <out-dir>/builtins.tgz, webapi.tgz, react.tgz, next.tgz, node.tgz, react-native.tgz
//
// What it packs is kept, `target/npm-crates/<key>/`, by a hash of all it's
// made from (`inputsKey`): the crates' files, the scripts that pack them
// and the toolchain. Cargo's packaging verifies each crate by building it
// afresh, 7 s of the pilot's 17 (2026-11-17), so a run whose inputs are
// unchanged copies the packs it made, the newest key's alone kept, a store
// `bun run disk` counts and caps (scripts/disk.ts).

import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { crates } from "../tooling/crates.js";
import { channel } from "./toolchain.ts";

const root = resolve(import.meta.dir, "..");
const args = Bun.argv.slice(2);
if (args.length !== 1 || args[0].startsWith("-")) throw new Error("Usage: bun scripts/package-npm-crates.ts <out-dir>");
const out = resolve(args[0]);

/** A hash of what the packs are made from: each crate's files Cargo
 * packages, `git ls-files` of its directory as `package-crates.ts` copies
 * them, as they are in the working tree; LICENSE; the two scripts and the
 * crates' catalog; and the toolchain, rust-toolchain.toml's channel,
 * which names the rustc and Cargo that verify each crate. */
function inputsKey(): string {
  const listed = Bun.spawnSync(["git", "ls-files", ...crates.map((crate) => crate.dir)], { cwd: root }).stdout.toString().split("\n").filter(Boolean);
  const files = [...listed, "LICENSE", "scripts/package-npm-crates.ts", "scripts/package-crates.ts", "tooling/crates.js"].sort();
  const hasher = new Bun.CryptoHasher("sha256");
  hasher.update(channel);
  for (const file of files) {
    hasher.update(`\0${file}\0`);
    if (existsSync(join(root, file))) hasher.update(readFileSync(join(root, file)));
  }
  return hasher.digest("hex").slice(0, 16);
}

const cacheRoot = join(root, "target", "npm-crates");
const cached = join(cacheRoot, inputsKey());
mkdirSync(out, { recursive: true });
if (existsSync(join(cached, "done"))) {
  for (const { dir } of crates) {
    copyFileSync(join(cached, `${dir}.tgz`), join(out, `${dir}.tgz`));
    console.log(join(out, `${dir}.tgz`));
  }
  process.exit(0);
}

/** Each crate released on npm, by its directory, and its package's name. */
const packages: Record<string, string> = Object.fromEntries(crates.map((crate) => [crate.dir, crate.npm]));

type Manifest = {
  package: {
    name: string;
    version: string;
    description: string;
    license: string;
    /** The npm packages a crate binds, `{ react = ">=18.0.0" }`: its package's peers too. */
    metadata?: { "rust-js"?: { npm?: Record<string, string> } };
  };
  dependencies?: Record<string, { package?: string; version?: string }>;
};

const run = (cmd: string[], cwd = root) => {
  const p = Bun.spawnSync(cmd, { cwd, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")} failed:\n${p.stderr.toString()}`);
  return p.stdout.toString();
};

const work = mkdtempSync(join(tmpdir(), "rust-js-npm-crates-"));
try {
  // Cargo's packaging, each crate verified, as crates.io would take it.
  const crates = join(work, "crates");
  run([process.execPath, "scripts/package-crates.ts", crates]);
  // The npm package each crate is, by the crate's name.
  const npmName = new Map(Object.entries(packages).map(([dir, name]) => [readManifest(join(root, dir, "Cargo.toml")).package.name, name]));
  mkdirSync(out, { recursive: true });
  for (const [dir, name] of Object.entries(packages)) {
    const { package: crate } = readManifest(join(root, dir, "Cargo.toml"));
    const file = readdirSync(crates).find((f) => f === `${crate.name}-${crate.version}.crate`);
    if (!file) throw new Error(`Cargo didn't package ${crate.name} ${crate.version}`);
    const unpacked = join(work, dir);
    mkdirSync(unpacked);
    run(["tar", "-xzf", join(crates, file), "-C", unpacked]);
    const staging = join(unpacked, `${crate.name}-${crate.version}`);
    // Cargo's record of how it packaged it, which isn't the crate's.
    for (const extra of ["Cargo.toml.orig", ".cargo_vcs_info.json", "Cargo.lock"]) rmSync(join(staging, extra), { force: true });
    copyFileSync(join(root, "LICENSE"), join(staging, "LICENSE"));
    // Its crates' requirements, as Cargo packaged them, are the peer
    // dependencies' ranges: `~0.0.1` means the same to both.
    const manifest = readManifest(join(staging, "Cargo.toml"));
    const peers = Object.fromEntries([
      ...Object.entries(manifest.dependencies ?? {}).flatMap(([key, dep]) => {
        const peer = npmName.get(dep.package ?? key);
        return peer && dep.version ? [[peer, dep.version]] : [];
      }),
      ...Object.entries(manifest.package.metadata?.["rust-js"]?.npm ?? {}),
    ]);
    const pkg = {
      name,
      version: crate.version,
      description: crate.description,
      license: crate.license,
      repository: { type: "git", url: "git+https://github.com/rust-js-lang/rust-js.git", directory: dir },
      keywords: ["rust-js", "rust"],
      "rust-js": { crate: crate.name },
      ...(Object.keys(peers).length ? { peerDependencies: peers } : {}),
      // Its types to TypeScript, webapi's event map (#1817).
      ...(existsSync(join(staging, "index.d.ts")) ? { types: "index.d.ts" } : {}),
      // What Cargo packaged, a build script and what it reads too.
      files: readdirSync(staging).filter((file) => file !== "package.json").sort(),
    };
    writeFileSync(join(staging, "package.json"), JSON.stringify(pkg, null, 2) + "\n");
    run([process.execPath, "pm", "pack", "--ignore-scripts", "--filename", join(out, `${dir}.tgz`)], staging);
    console.log(join(out, `${dir}.tgz`));
  }
  // Kept for the next run of the same inputs: written aside, then put in
  // place whole, as another run may be packing at once; the older keys go.
  mkdirSync(cacheRoot, { recursive: true });
  for (const old of readdirSync(cacheRoot)) rmSync(join(cacheRoot, old), { recursive: true, force: true });
  const aside = mkdtempSync(join(cacheRoot, ".packing-"));
  for (const dir of Object.keys(packages)) copyFileSync(join(out, `${dir}.tgz`), join(aside, `${dir}.tgz`));
  writeFileSync(join(aside, "done"), "");
  try {
    renameSync(aside, cached);
  } catch {
    rmSync(aside, { recursive: true, force: true });
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}

function readManifest(path: string): Manifest {
  return Bun.TOML.parse(readFileSync(path, "utf8")) as Manifest;
}
