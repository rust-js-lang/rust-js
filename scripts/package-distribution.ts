// Assemble a local distribution. No registry publication or global installation.
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { parseCompilerIdentity } from "../tooling/manifest.js";
import { crates } from "../tooling/crates.js";

const root = resolve(import.meta.dir, "..");
const [compilerPath, destination, ...extra] = Bun.argv.slice(2);
if (!compilerPath || !destination || extra.length) throw new Error("Usage: bun scripts/package-distribution.ts <compiler> <new-output-directory>");
const compiler = resolve(compilerPath);
const output = resolve(destination);
if (existsSync(output)) throw new Error(`Output already exists: ${output}; choose a new distribution directory`);
const run = (args: string[], cwd = root) => {
  const result = Bun.spawnSync(args, { cwd, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  return result.stdout.toString();
};
const identity = parseCompilerIdentity(run([compiler, "--version-json"]));
// Each package of the hosts', by the directory it's packed from and the
// tarball it's packed as: with the compiler's and the crates', what
// `distribution.json` names, the one list of a distribution's files.
const hosts = [["tooling", "build.tgz"], ["runtime", "runtime.tgz"], ["std", "std.tgz"], ["typescript", "typescript.tgz"]];
const named = (directory: string) => JSON.parse(readFileSync(join(root, directory, "package.json"), "utf8"));
for (const [directory] of hosts) {
  const pkg = named(directory);
  if (pkg.version !== identity.version) throw new Error(`${pkg.name} ${pkg.version} does not match compiler ${identity.version}`);
}
// The commit it's made from, and whether the checkout had changes of its
// own, before anything is made in it, so what qualifies it can be run from
// the same one.
const source = { commit: run(["git", "rev-parse", "HEAD"]).trim(), clean: run(["git", "status", "--porcelain"]).trim() === "" };
mkdirSync(dirname(output), { recursive: true });
const staging = mkdtempSync(join(dirname(output), ".rust-js-distribution-"));
try {
  // @rust-js/runtime and @rust-js/std, what the JS the compiler writes
  // imports (ADR 0103, 0552).
  for (const [directory, file] of hosts) {
    run([process.execPath, "pm", "pack", "--ignore-scripts", "--filename", join(staging, file)], join(root, directory));
  }
  run([process.execPath, "scripts/package-compiler.ts", compiler, join(staging, "native.tgz")]);
  // The crates an app's Cargo builds with, as npm has them (ADR 0118).
  run([process.execPath, "scripts/package-npm-crates.ts", staging]);
  const sha256 = (file: string) => createHash("sha256").update(readFileSync(join(staging, file))).digest("hex");
  // Each by its package, which an app installs it as.
  const artifacts = [
    ...hosts.map(([directory, file]) => ({ file, package: named(directory).name as string })),
    { file: "native.tgz", package: "@rust-js/native" },
    ...crates.map(({ dir, npm }) => ({ file: `${dir}.tgz`, package: npm })),
  ].map((artifact) => ({ ...artifact, sha256: sha256(artifact.file) }));
  const files = artifacts.map(({ file }) => file);
  writeFileSync(join(staging, "distribution.json"), JSON.stringify({
    version: 1, compiler: identity, source, platform: process.platform, arch: process.arch, artifacts,
  }, null, 2) + "\n");
  writeFileSync(join(staging, "SHA256SUMS"), [...files, "distribution.json"].map(file => `${sha256(file)}  ${file}\n`).join(""));
  // The destination must be new; never replace a previous bundle on failure.
  if (existsSync(output)) throw new Error(`Output already exists: ${output}`);
  renameSync(staging, output);
  console.log(output);
} finally {
  rmSync(staging, { recursive: true, force: true });
}
