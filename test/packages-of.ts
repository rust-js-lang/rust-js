// Which `@rust-js/runtime` and `@rust-js/std` a run of the tests' JS
// imports (ADRs 0103, 0561): the checkout's, or a target's own. A runtime
// helper's mutant has its own runtime, as it has its own compiler, so its
// runner installs that runtime in the mutant's target, with a copy of the
// checkout's std, whose imports of the runtime resolve beside it. Every
// program a test writes in that target resolves them before the
// checkout's, which is never rewritten: a `check:fast` beside the
// mutant's tests read its runtime as the checkout's (#720). Node runs this
// too, as the oracle's scripts import it, so it uses no Bun API.
import { cpSync, existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));

// The checkout's own target, whatever `RUST_JS_TARGET` says: where what's
// kept by its sources alone is, which no build of the compiler changes, a
// native program's wrapper and its builds (`nativeBinary`, test/support.ts).
// A mutant's tests and each test the test map records run in a target of
// their own, and kept a native program by where its wrapper was there: each
// built every one it ran again, the map's 3,642 at each recording, left
// behind, 2 GB of a store held to 3 GB (#875).
export const checkoutTarget = join(root, "target");

// The tests' target, or with `RUST_JS_TARGET`, another: a mutant's tests',
// which build the crates with its compiler where no other test reads them
// (ADR 0093).
export const target = process.env.RUST_JS_TARGET ? resolve(process.env.RUST_JS_TARGET) : checkoutTarget;

/** Where a target's own packages are, `node_modules/@rust-js/` in it. */
function installed(at: string): string {
  return join(at, "node_modules", "@rust-js");
}

/** The runtime's and std's packages the JS written in `at` imports: the
 * target's own where it has them, or the checkout's. */
function packagesOf(at: string): { runtime: string; std: string } {
  const own = installed(at);
  return existsSync(join(own, "runtime"))
    ? { runtime: join(own, "runtime"), std: join(own, "std") }
    : { runtime: join(root, "runtime"), std: join(root, "std") };
}

/** The packages this run's JS imports: `target`'s. */
export const { runtime: runtimeDir, std: stdDir } = packagesOf(target);

/** Packages of `at`'s own, `runtime` its runtime's module, and the
 * checkout's std copied, not linked: Node resolves a module's imports from
 * its real path, so a link would import the checkout's runtime. */
export function installPackages(at: string, runtime: string) {
  const own = installed(at);
  rmSync(own, { recursive: true, force: true });
  mkdirSync(join(own, "runtime"), { recursive: true });
  cpSync(join(root, "runtime", "package.json"), join(own, "runtime", "package.json"));
  writeFileSync(join(own, "runtime", "index.js"), runtime);
  cpSync(join(root, "std"), join(own, "std"), { recursive: true });
}
