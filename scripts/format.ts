// This repository's Rust, formatted by `rust-js fmt` (#1901), the one way:
// rustfmt's, then the `jsx!` bodies'. With no files, the Cargo workspace's,
// then those no target of it declares that rustfmt formats too: the
// playground's, the pilot's and the scripts' crates. `--check` formats
// nothing, prints each file's diff and exits 1 where one would change.
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const args = Bun.argv.slice(2);
const check = args.includes("--check");
const files = args.filter(arg => arg !== "--check");
if (files.some(file => file.startsWith("-"))) {
  console.error("Usage: bun run fmt [--check] [file.rs ...]");
  process.exit(1);
}

function run(command: string[], env: Record<string, string | undefined> = process.env): { code: number; out: string } {
  const result = Bun.spawnSync(command, { cwd: root, env, stdin: "ignore", stdout: "pipe", stderr: "inherit" });
  return { code: result.exitCode ?? 1, out: result.stdout.toString() };
}

// The compiler given, as an installed one is qualified with (ADR 0094),
// or this checkout's, built first: as a shell builds it, with
// `.cargo/config.toml`'s `RUSTC_BOOTSTRAP`, not a test's `.env.test`'s,
// which would rebuild what the tests built.
const given = process.env.RUST_JS_COMPILER;
if (!given && run(["cargo", "build", "--locked", "--quiet"], { ...process.env, RUSTC_BOOTSTRAP: undefined }).code !== 0) {
  process.exit(1);
}
const compiler = given ? resolve(given) : resolve(root, "target/debug/rust-js");
const flags = check ? ["--check"] : [];
const extra = () => run([
  "git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", "wasm/web/rust/*.rs", "examples/pilot/*.rs", "scripts/crate-corpus/*.rs", "scripts/metamorph/src/*.rs",
]).out.split("\0").filter(Boolean);
const runs = files.length > 0 ? [files] : [[], extra()];
let failed = false;
for (const paths of runs) {
  const result = run([compiler, "fmt", ...flags, ...paths]);
  process.stdout.write(result.out);
  failed ||= result.code !== 0;
}
if (failed) process.exitCode = 1;
