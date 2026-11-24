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
// One heavy run at a time (Tu's rule, 2026-11-24, where agents share a
// machine): a lease is taken only with the machine's turn (`turnLock`),
// waited for while another run holds it, so a check, a push, a mutation run
// or a sweep runs alone, and what agents beside it do meanwhile is light,
// reading and single tests. A lane's run (`RUST_JS_LANE`), the background
// work, starts only where no run holds a lease and takes no turn, so a run
// started after it goes on at once beside it, the lane at its lower
// priority, its share not counted against the run's (`sharers`).
//
// The turn goes by priority, not to whichever waiter polls first after it's
// let go (#2516): each waiter says what it is and since when in the
// waiters' store (`waiterStore`), and takes the turn only where it's next
// (`nextWaiter`): a push first, as a red `main`'s fix is one, then the
// longest waiting. A long run, a mutation run's batches, a retirement's
// names, lets the turn go between them where a push waits (`yieldToPush`),
// and waits for it again, where it was in the queue: a push of a red
// `main`'s fix waited 34 minutes behind one retirement, then behind the next.
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
export type Lease = { pid: number; checkout: string; kind: string; lane?: true };

/** The machine's turn of `store`'s leases: one run's at a time. */
export const turnLock = (store = leaseStore) => `${store}.turn`;

/** Where the machine's waiters for its turn are, one file each, by its
 * process: what it is and since when it waits. */
export const waiterStore = (store = leaseStore) => `${store}.waiting`;

/** A run waiting for the machine's turn: its process, what it is, and when
 * it began to wait. */
export type Waiter = { pid: number; kind: string; since: number };

/** Of `waiters`, whose the turn is next: a push's first, then the longest
 * waiting, the lower process on a tie (#2516). */
export function nextWaiter(waiters: Waiter[]): Waiter | undefined {
  const rank = (waiter: Waiter) => (waiter.kind === "push" ? 0 : 1);
  return [...waiters].sort((a, b) => rank(a) - rank(b) || a.since - b.since || a.pid - b.pid)[0];
}

/** The live waiters of `store`'s turn, each whose process is gone taken
 * away. */
export function liveWaiters(store = leaseStore): Waiter[] {
  const dir = waiterStore(store);
  if (!existsSync(dir)) return [];
  const waiters: Waiter[] = [];
  for (const name of readdirSync(dir)) {
    if (!name.endsWith(".json")) continue;
    let waiter: Waiter | undefined;
    try {
      waiter = JSON.parse(readFileSync(join(dir, name), "utf8"));
    } catch {
      continue;
    }
    if (waiter && alive(waiter.pid)) waiters.push(waiter);
    else rmSync(join(dir, name), { force: true });
  }
  return waiters;
}

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
 * where what started it holds one (`RUST_JS_LEASED`, the holder's process),
 * being in its share. Taken with the machine's turn, waited for while
 * another run holds it; a lane's, where no run holds a lease, no turn
 * taken. What this process starts inherits it: `RUST_JS_LEASED` set. Its
 * wait for them, in ms, which a push's line says (`turn`, #2516). */
export function holdLease(
  kind: string,
  { store = leaseStore, env = process.env, say = (line: string) => console.log(line), every = 60_000 }: { store?: string; env?: NodeJS.ProcessEnv; say?: (line: string) => void; every?: number } = {},
): number {
  if (env.RUST_JS_LEASED) return 0;
  const asked = Date.now();
  mkdirSync(store, { recursive: true });
  const lane = Boolean(env.RUST_JS_LANE);
  let releaseTurn: (() => void) | undefined;
  if (lane) {
    const began = Date.now();
    let said: number | undefined;
    while (liveLeases(store).some((lease) => lease.pid !== process.pid) || lockState(turnLock(store)) === "held") {
      if (said === undefined || Date.now() - said >= every) {
        say(`${kind}: a lane's run, waiting until no other run holds the machine${said === undefined ? "" : `: ${Math.round((Date.now() - began) / 60_000)} min so far`}`);
        said = Date.now();
      }
      Bun.sleepSync(1000);
    }
  }
  const since = Date.now();
  if (!lane) releaseTurn = takeTurn(kind, since, { store, say, every });
  const at = join(store, `${process.pid}.json`);
  const lease: Lease = { pid: process.pid, checkout, kind, ...(lane ? { lane: true as const } : {}) };
  writeLease(at, lease);
  env.RUST_JS_LEASED = String(process.pid);
  if (!lane) held = { kind, since, store, at, lease, releaseTurn: releaseTurn!, say, every };
  const release = () => {
    rmSync(at, { force: true });
    if (held) held.releaseTurn();
    else releaseTurn?.();
  };
  process.on("exit", release);
  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    process.once(signal, () => {
      release();
      process.exit(128 + (signal === "SIGINT" ? 2 : signal === "SIGTERM" ? 15 : 1));
    });
  }
  return Date.now() - asked;
}

/** `lease` written whole at `at`. */
function writeLease(at: string, lease: Lease) {
  writeFileSync(`${at}.writing`, JSON.stringify(lease));
  rmSync(at, { force: true });
  renameSync(`${at}.writing`, at);
}

/** The machine's turn of `store` taken by this process, a run of `kind`
 * waiting since `since`: its waiter written, the turn taken once it's free
 * and this the next waiter (`nextWaiter`), the waiter then removed. What
 * lets the turn go. */
function takeTurn(kind: string, since: number, { store, say, every }: { store: string; say: (line: string) => void; every: number }): () => void {
  const dir = waiterStore(store);
  mkdirSync(dir, { recursive: true });
  const mine = join(dir, `${process.pid}.json`);
  writeFileSync(`${mine}.writing`, JSON.stringify({ pid: process.pid, kind, since } satisfies Waiter));
  renameSync(`${mine}.writing`, mine);
  const holder = () => {
    try {
      return readFileSync(join(turnLock(store), "owner"), "utf8").trim();
    } catch {
      return "";
    }
  };
  try {
    return acquire(turnLock(store), {
      wait: Number.POSITIVE_INFINITY,
      waiting: `${kind}: waiting for the machine's other heavy run, process ${holder()}, one at a time (ADR 1146)`,
      every,
      say,
      ahead: () => nextWaiter(liveWaiters(store))?.pid !== process.pid,
    })!;
  } finally {
    rmSync(mine, { force: true });
  }
}

/** This process's turn and lease, where it holds them (`holdLease`). */
let held: { kind: string; since: number; store: string; at: string; lease: Lease; releaseTurn: () => void; say: (line: string) => void; every: number } | undefined;

/** Whether a push waits for the machine's turn this process holds. */
export function pushWaits(): boolean {
  return held !== undefined && liveWaiters(held.store).some((waiter) => waiter.kind === "push" && waiter.pid !== process.pid);
}

/** Of a long run, between its batches, with nothing of its own running:
 * where a push waits for the turn this process holds (`pushWaits`), the
 * turn and the lease let go, so the push runs, and both taken again, this
 * run waiting as it first did, since then, so it's next after the push and
 * before any run that began to wait after it. Never mid-batch, so no work
 * is lost (#2516). Whether it yielded. */
export function yieldToPush(): boolean {
  if (held === undefined || !pushWaits()) return false;
  const turn = held;
  turn.say(`${turn.kind}: a push waits for the machine's turn: letting it go between batches, and waiting for it again (#2516)`);
  rmSync(turn.at, { force: true });
  turn.releaseTurn();
  turn.releaseTurn = takeTurn(turn.kind, turn.since, turn);
  writeLease(turn.at, turn.lease);
  return true;
}

/** Of a run's items in turn, a retirement's names, before the one at `at`:
 * where a push waits for the turn this run holds, `before`, what of the
 * run's is still running, awaited, and the turn let go to the push and
 * waited for again (`yieldToPush`); never before the first. Whether it
 * yielded (#2516). */
export async function yieldBetween(at: number, before: () => Promise<unknown>): Promise<boolean> {
  if (at === 0 || !pushWaits()) return false;
  await before();
  return yieldToPush();
}

/** `queue`'s batches run by `workers` at once, `one` each, a worker with
 * nothing to take waiting while another's may yet add to it, a build that
 * splits in two: a mutation run's pool. Where a push waits for the turn this
 * run holds, no worker takes another batch: once those running end, and
 * `beside`, what runs beside them, the turn goes to the push and is waited
 * for again, and the run goes on, no work lost (#2516). */
export async function runYielding<T>(
  queue: T[],
  workers: number,
  one: (item: T, slot: number) => Promise<void>,
  { beside = async () => {}, poll = 1000 }: { beside?: () => Promise<unknown>; poll?: number } = {},
): Promise<void> {
  let busy = 0;
  let yielding: Promise<void> | undefined;
  const pause = () => new Promise((resume) => setTimeout(resume, poll));
  const between = async () => {
    while (busy > 0) await pause();
    await beside();
    yieldToPush();
  };
  await Promise.all(
    Array.from({ length: Math.min(workers, queue.length) }, async (_, slot) => {
      for (;;) {
        if (yielding === undefined && queue.length > 0 && pushWaits()) yielding = between().finally(() => (yielding = undefined));
        if (yielding !== undefined) {
          await yielding;
          continue;
        }
        const next = queue.shift();
        if (next === undefined) {
          if (busy === 0) return;
          await pause();
          continue;
        }
        busy++;
        try {
          await one(next, slot);
        } finally {
          busy--;
        }
      }
    }),
  );
}

/** How many runs share the machine with this one, itself among them: the
 * live leases but its own and its holder's (`RUST_JS_LEASED`), and, of a
 * run not a lane's, but the lanes', whose lower priority yields to it. */
export function sharers(env = process.env, store = leaseStore): number {
  const own = new Set([process.pid, Number(env.RUST_JS_LEASED)]);
  const lane = Boolean(env.RUST_JS_LANE);
  return liveLeases(store).filter((lease) => !own.has(lease.pid) && (lane || !lease.lane)).length + 1;
}

/** A run's share of `whole`, the machine's cores or memory: divided by the
 * live leases, this one's among them, at least one. */
export function fairShare(whole: number, leases = liveLeases().length): number {
  return whole / Math.max(1, leases);
}

/** The part of the machine a run started by itself sizes by, its cores
 * and its memory alike: its `fairShare` of one; or all of it, of one a
 * pool started (`RUST_JS_LEASED`), the pool's share holding it already. A
 * run that took its cores' share and the whole memory budget, as mutation
 * runs did, sized its builds and tests for memory its neighbours held too
 * (#1767). */
export function ownPart(env = process.env, leases?: number): number {
  return env.RUST_JS_LEASED ? 1 : fairShare(1, leases ?? sharers(env));
}

/** A run's own whole cores of `whole`, at least one (`ownPart`). */
export function ownShare(whole: number, env = process.env, leases?: number): number {
  return Math.max(1, Math.floor(whole * ownPart(env, leases)));
}

/** A run's own GB of `budget`, the machine's memory budget (`ownPart`). */
export function ownMemory(budget: number, env = process.env, leases?: number): number {
  return budget * ownPart(env, leases);
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
 * ms, `waiting` said as it starts to and again each `every` ms, with how
 * long and whose process holds it, by `say`: its wait sleeps the process,
 * so no timer of the run's says anything meanwhile, and a mutation run
 * that waited 50 minutes said it once (#2418). Past that, `undefined`. */
export function acquire(
  lock: string,
  {
    wait = 30_000,
    waiting,
    every = 60_000,
    say = (line: string) => console.log(line),
    ahead,
  }: { wait?: number; waiting?: string; every?: number; say?: (line: string) => void; ahead?: () => boolean } = {},
  stateOf: (lock: string) => LockState = lockState,
): (() => void) | undefined {
  const owner = join(lock, "owner");
  const began = Date.now();
  const until = began + wait;
  let said: number | undefined;
  for (;;) {
    try {
      // Another's turn first, where one waits ahead of this (`nextWaiter`):
      // waited for as a lock held is.
      if (ahead?.()) throw Object.assign(new Error("ahead"), { code: "AHEAD" });
      mkdirSync(lock, { recursive: false });
      writeFileSync(owner, String(process.pid));
      return () => rmSync(lock, { recursive: true, force: true });
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (code !== "EEXIST" && code !== "AHEAD") {
        mkdirSync(join(lock, ".."), { recursive: true });
        continue;
      }
      // Gone since `mkdir` refused it, its holder done: made again. Only
      // a stale one is removed: an absent one taken for stale removed what
      // another run made in that instant, and both held it, two job-time
      // writers each keeping what it read (#2283).
      const state = stateOf(lock);
      if (code === "EEXIST" && state === "absent") continue;
      if (state === "stale") {
        rmSync(lock, { recursive: true, force: true });
        continue;
      }
      if (Date.now() > until) return undefined;
      if (waiting && said === undefined) {
        say(waiting);
        said = Date.now();
      } else if (waiting && said !== undefined && Date.now() - said >= every) {
        let holder = "";
        try {
          holder = `, held by process ${readFileSync(owner, "utf8").trim()}`;
        } catch {}
        say(`${waiting}: ${Math.round((Date.now() - began) / 60_000)} min so far${holder}`);
        said = Date.now();
      }
      Bun.sleepSync(25);
    }
  }
}

/** Whether `lock` is held (`acquire`'s): made, and its owner's process
 * alive, or made under 5 s ago naming no owner yet. What `acquire` waits
 * on, and what a store a run works in is kept from the pruner by while it
 * runs, `target/mutants` by its run's `mutants.lock` (#2272). */
/** What `lock` is: `absent`, not made; `held`, made and its owner's
 * process alive, or made under 5 s ago naming no owner yet; else `stale`,
 * its owner gone, which `acquire` alone removes (#2283). */
export type LockState = "absent" | "held" | "stale";
export function lockState(lock: string): LockState {
  let made: number;
  try {
    made = statSync(lock).mtimeMs;
  } catch {
    return "absent";
  }
  let held: number | undefined;
  try {
    held = Number(readFileSync(join(lock, "owner"), "utf8"));
  } catch {}
  const live = held !== undefined && Number.isFinite(held) && held > 0 ? alive(held) : Date.now() - made <= 5_000;
  return live ? "held" : "stale";
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
