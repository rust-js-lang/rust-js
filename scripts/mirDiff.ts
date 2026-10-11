// A program's JS from THIR and from MIR (ADR 0364), side by side: each
// compiled, then their difference, which MIR's lowering is to make none.
// With React, `bun run test`'s build of it, for JSX.
//
//   bun scripts/mirDiff.ts lib.rs           the difference, or "same"
//   bun scripts/mirDiff.ts lib.rs --mir     MIR's JS alone
import { mkdtempSync, readFileSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = join(import.meta.dir, "..");
const target = join(root, "target");
const [file, ...flags] = process.argv.slice(2);
if (!file) throw new Error("usage: bun scripts/mirDiff.ts <file.rs> [--mir]");
const source = resolve(file);
const crates = ["react", "webapi", "js"].flatMap((name) => ["--extern", `${name}=${join(target, `lib${name}.rmeta`)}`]);

const compile = (mode: "thir" | "mir") => {
  const dir = mkdtempSync(join(tmpdir(), `mir-diff-${mode}-`));
  const p = Bun.spawnSync(
    [join(target, "debug", "rust-js"), source, "-o", join(dir, "lib.js"), "--manifest", join(dir, "manifest.json"), "--", ...crates, "-L", target],
    { env: { ...process.env, RUST_JS_MIR: mode === "mir" ? "1" : "0" }, stderr: "pipe" },
  );
  const errors = p.stderr.toString().split("\n").filter((line) => line.startsWith("error"));
  if (p.exitCode !== 0) return { errors, js: "" };
  const out = readdirSync(dir).find((name) => /^lib\.(js|jsx)$/.test(name))!;
  // What names the file it was compiled from, and its map, differs alone.
  const js = readFileSync(join(dir, out), "utf8").split("\n").filter((line) => !line.startsWith("//")).join("\n");
  return { errors, js };
};

const mir = compile("mir");
if (flags.includes("--mir")) {
  console.log(mir.errors.length > 0 ? mir.errors.join("\n") : mir.js);
  process.exit(0);
}
const thir = compile("thir");
for (const [mode, result] of [["THIR", thir], ["MIR", mir]] as const) {
  if (result.errors.length > 0) console.log(`${mode}:\n${result.errors.join("\n")}`);
}
if (thir.js === mir.js) {
  console.log("same");
} else {
  const dir = mkdtempSync(join(tmpdir(), "mir-diff-"));
  await Bun.write(join(dir, "thir.js"), thir.js);
  await Bun.write(join(dir, "mir.js"), mir.js);
  const diff = Bun.spawnSync(["git", "diff", "--no-index", "--no-color", join(dir, "thir.js"), join(dir, "mir.js")]);
  console.log(diff.stdout.toString().split("\n").slice(4).join("\n"));
}
