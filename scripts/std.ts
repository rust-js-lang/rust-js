// std's bodies compiled into `@rust-js/std` (ADR 0561), the one way:
// `bun run std` writes `std/iter.js`, its map and `std/manifest.json`;
// `test/std-package.test.ts` compiles them again elsewhere and compares.
//
//   bun scripts/std.ts [<root>]   # <root>/std/lib.rs into <root>/std/, the checkout's unless it's given
//
// Run in the root, the crate `std/lib.rs`, as its modules' first lines name
// it. Its manifest's crate hash is of its inputs alone, the same wherever and
// however it's built (`Manifest::of_its_inputs`, #1094).
import { spawnSync } from "node:child_process";
import { join, resolve } from "node:path";

/** The command that compiles `<root>/std/lib.rs` into `<root>/std/` with
 * `compiler`, and where it's run, `<root>`. */
export function stdBuild(compiler: string, root: string): { cmd: string[]; cwd: string } {
  const cmd = [compiler, "std/lib.rs", "-o", "std/index.js", "--library", "--manifest", "std/manifest.json", "--std-bodies", "--", "--crate-name", "rust_js_std"];
  return { cmd, cwd: resolve(root) };
}

if (import.meta.main) {
  const checkout = join(import.meta.dir, "..");
  const { cmd, cwd } = stdBuild(join(checkout, "target", "debug", "rust-js"), process.argv[2] ?? checkout);
  const p = spawnSync(cmd[0], cmd.slice(1), { cwd, stdio: "inherit" });
  process.exit(p.status ?? 1);
}
