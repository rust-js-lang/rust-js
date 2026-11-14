// What concurrent runs share of the machine, and the locks of what they
// write together: checkouts' checks and mutation runs, at once on one
// machine, in two worktrees or in one.
//
// A run that sizes itself by the machine holds a lease while it runs
// (`holdLease`), in the machine's store, which every worktree shares. Each
// takes its share, the machine's cores and memory divided by the live
// leases (`fairShare`), asked again as its jobs start: two checks started
// together took each every core, the load 67 on 14 cores, each 60% slower
// than alone. A share counts runs, not the load, so two settle at half each
// and neither swings with the other's load. What a run starts inherits its
// lease (`RUST_JS_LEASED`) and holds none of its own, being in its share.
//
// A lock (`withLock`) serializes what two runs read, change and write
// again, the machine's job times, or keeps one at a time where only one
// can work, a checkout's mutants' build. Its owner is a process: one whose
// process is gone, killed or crashed, holds nothing, and the next takes it.
import { existsSync, mkdirSync, readdirSync, readFileSync, realpathSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkout, machineStore } from "../test/bounded";

/** Where the machine's leases are, one file each, by its process. */
export const leaseStore = join(machineStore, "leases");

/** A run's lease: its process, its checkout, what it is. */
export type Lease = { pid: number; checkout: string; kind: string };

/** Whether `pid` is a live process. */
export function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    // Another user's, which this one may not signal, lives.
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

/** The live leases of `store`, each whose process is gone taken away. */
export function liveLeases(store = leaseStore): Lease[] {
  if (!existsSync(store)) return [];
  const leases: Lease[] = [];
  for (const name of readdirSync(store)) {
    const at = join(store, name);
    let lease: Lease | undefined;
    try {
      lease = JSON.parse(readFileSync(at, "utf8"));
    } catch {
      // Being written, or not a lease: neither counts.
      continue;
    }
    if (lease && alive(lease.pid)) leases.push(lease);
    else rmSync(at, { force: true });
  }
  return leases;
}

/** This process's lease of `kind` held until it exits, in `store`: none
 * where what started it holds one (`RUST_JS_LEASED`), being in its share. */
export function holdLease(kind: string, { store = leaseStore, env = process.env }: { store?: string; env?: NodeJS.ProcessEnv } = {}): void {
  if (env.RUST_JS_LEASED) return;
  mkdirSync(store, { recursive: true });
  const at = join(store, `${process.pid}.json`);
  const lease: Lease = { pid: process.pid, checkout, kind };
  writeFileSync(`${at}.writing`, JSON.stringify(lease));
  rmSync(at, { force: true });
  renameSync(`${at}.writing`, at);
  const release = () => rmSync(at, { force: true });
  process.on("exit", release);
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.once(signal, () => {
      release();
      process.exit(128 + (signal === "SIGINT" ? 2 : signal === "SIGTERM" ? 15 : 1));
    });
  }
}

/** A run's share of `whole`, the machine's cores or memory: divided by the
 * live leases, this one's among them, at least one. */
export function fairShare(whole: number, leases = liveLeases().length): number {
  return whole / Math.max(1, leases);
}

/** What a run started by itself sizes by of `whole`: its `fairShare`; or
 * all of it, of one a pool started (`RUST_JS_LEASED`), the pool's share
 * holding it already. Whole cores, at least one. */
export function ownShare(whole: number, env = process.env): number {
  return Math.max(1, Math.floor(env.RUST_JS_LEASED ? whole : fairShare(whole)));
}

/** Whether any live lease is of `dir`'s checkout: its stores are in use. */
export function leasedIn(dir: string, store = leaseStore): boolean {
  const real = (path: string) => {
    try {
      return realpathSync(path);
    } catch {
      return path;
    }
  };
  return liveLeases(store).some((lease) => real(lease.checkout) === real(dir));
}

/** `lock` taken, a directory made at once or not at all, and what lets it
 * go: one whose owner's process is gone is taken, as is one whose maker
 * named no owner 5 s past making it, gone between. Waited for up to `wait`
 * ms, `waiting` said once if it must; past that, `undefined`. */
export function acquire(lock: string, { wait = 30_000, waiting }: { wait?: number; waiting?: string } = {}): (() => void) | undefined {
  const owner = join(lock, "owner");
  const until = Date.now() + wait;
  let said = false;
  for (;;) {
    try {
      mkdirSync(lock, { recursive: false });
      writeFileSync(owner, String(process.pid));
      return () => rmSync(lock, { recursive: true, force: true });
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") {
        mkdirSync(join(lock, ".."), { recursive: true });
        continue;
      }
      let held: number | undefined;
      try {
        held = Number(readFileSync(owner, "utf8"));
      } catch {}
      let made = Date.now();
      try {
        made = statSync(lock).mtimeMs;
      } catch {}
      const gone = held !== undefined && Number.isFinite(held) && held > 0 ? !alive(held) : Date.now() - made > 5_000;
      if (gone) {
        rmSync(lock, { recursive: true, force: true });
        continue;
      }
      if (Date.now() > until) return undefined;
      if (waiting && !said) {
        console.log(waiting);
        said = true;
      }
      Bun.sleepSync(25);
    }
  }
}

/** `run` with `lock` held (`acquire`), let go as it ends; past `wait`,
 * `undefined`, and `run` not run. */
export function withLock<T>(lock: string, run: () => T, { wait = 30_000 }: { wait?: number } = {}): T | undefined {
  const release = acquire(lock, { wait });
  if (!release) return undefined;
  try {
    return run();
  } finally {
    release();
  }
}
