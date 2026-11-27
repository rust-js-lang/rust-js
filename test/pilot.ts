// The pilot (ROADMAP M3.3, `examples/pilot`), as its two test files make it:
// test/pilot.test.ts runs it, the server, Vite and a browser; and
// test/pilot-committed.test.ts checks it as it's committed, its manifests
// and its JS, with no server, no Vite and no browser, so a bless of its JS,
// or a mutation's run of that check, waits on none of them (#2524). Each
// file's pilot is a kept app of its own, and compiles the client once.

import { existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { writePatch } from "../tooling/patch.js";
import { fixture, KeptApp, root, run, stdDir } from "./support";

export const checkout = join(root, "examples", "pilot");

/** The limit of a step of a pilot's setup, its packs or a build: about
 * twice the whole setup's time cold in CI, where nothing's built (29 s,
 * nightly 45739620208), as long as the machine's load makes it. A step
 * that hangs ends in a minute or two and says which it was, where each had
 * 300 s or 600 s, and a hang was the file's longest step (#2524). */
export const setupLimit = () => underLoad(60_000);

/** The pilot, copied under `target/`, where its build writes its JS beside
 * its Rust (ADR 0041): never into the checkout, whatever compiler a test is
 * given. Kept between runs, its Cargo target in it, so a run that changes
 * none of what it's built from finds it built (`KeptApp`, #882). */
export function keptPilot(): KeptApp {
  const kept = new KeptApp("pilot");
  kept.copy(checkout, "", (path) => !/(^|[/\\])(target|node_modules|dist|\.cargo)([/\\]|$)/.test(path));
  return kept;
}

/** The pilot is an app of its own, which names its packages by version
 * (ROADMAP M3.3): installed as a package manager installs them, this
 * checkout's, and Cargo told where its crates are by the patch `rust-js
 * build` writes. */
export function install(kept: KeptApp) {
  const packs = fixture("pilot-crates");
  // Kept by what they're made from, 0.02 s; made again, each of the six
  // crates verified by a build of its own, 7 s on a quiet Mac (docs/dev-loop.md).
  run([process.execPath, "scripts/package-npm-crates.ts", packs], setupLimit());
  // Each unpacked beside, then written where it differs: a crate's files
  // unpacked anew are new to Cargo, which builds it again.
  for (const name of ["builtins", "webapi", "react"]) {
    const unpacked = join(packs, name);
    mkdirSync(unpacked, { recursive: true });
    run(["tar", "-xzf", join(packs, `${name}.tgz`), "-C", unpacked, "--strip-components", "1"]);
    kept.copy(unpacked, join("node_modules", "@rust-js", name));
  }
  // rust-js's JS packages, and the libraries, as this checkout has them.
  const linked: [string, string][] = [
    ["@rust-js/std", stdDir],
    ["@rust-js/build", join(root, "tooling")],
    ...["react", "react-dom", "sonner", "vite", "@vitejs/plugin-react"].map((name): [string, string] => [name, join(root, "node_modules", name)]),
  ];
  for (const [name, target] of linked) {
    // A link to nothing would be a page that never loads, each test's.
    if (!existsSync(target)) throw new Error(`${name} isn't installed in this checkout: run \`bun install\``);
    kept.link(join("node_modules", name), target);
  }
  kept.done();
  writePatch(kept.dir);
}

/** The JS beside a pilot's Rust, by path. */
export function writtenJs(dir: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const crate of ["frontend", "models"]) {
    const src = join(dir, crate, "src");
    for (const name of readdirSync(src).filter((n) => /\.jsx?$/.test(n))) found.set(join(crate, "src", name), readFileSync(join(src, name), "utf8"));
  }
  return found;
}
