// A run's own compiler (#761, #2317): the one way a run that holds the
// checkout's debug compiler while builds may run takes it. A `cargo build`
// in the checkout meanwhile, an agent's next edit's or another step's,
// replaces `target/debug/rust-js`: it's gone for part of that, 899 of
// 88,241 looks during one build (#761), and what spawns it then fails at
// once ("could not execute .. No such file", a push's React port's, #2317).
// Cargo writes a relinked compiler as a new file, so a hard link of the one
// built stays that build, whatever the checkout's becomes.
//
// Each is `target/<kind>/<pid>/<name>`, its folder gone as its process
// exits; those of runs whose process is gone, killed before they could
// remove theirs, go as the next run of the kind takes its own.

import { linkSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";

const folders = new Map<string, string>();

/** The folder `kind`'s runs keep their compilers in, this process's, made
 * once: what else the run keeps for itself may go in it too. */
export function ownFolder(root: string, kind: string): string {
  const runs = join(root, "target", kind);
  const made = folders.get(runs);
  if (made !== undefined) return made;
  mkdirSync(runs, { recursive: true });
  for (const pid of readdirSync(runs)) {
    try {
      process.kill(Number(pid), 0);
    } catch {
      rmSync(join(runs, pid), { recursive: true, force: true });
    }
  }
  const own = join(runs, String(process.pid));
  mkdirSync(own, { recursive: true });
  process.on("exit", () => rmSync(own, { recursive: true, force: true }));
  folders.set(runs, own);
  return own;
}

/** This process's own link of the checkout's debug compiler as it's built
 * now, `name` in `kind`'s folder of it: a link of an earlier build by the
 * same name is replaced, what still runs it keeping its own. */
export function ownCompiler(root: string, kind: string, name = "rust-js"): string {
  const compiler = join(ownFolder(root, kind), name);
  rmSync(compiler, { force: true });
  linkSync(join(root, "target", "debug", "rust-js"), compiler);
  return compiler;
}
