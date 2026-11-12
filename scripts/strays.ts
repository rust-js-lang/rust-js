// Processes the loop left running that nothing will stop: a run of a
// case's JS, a native program, a compile, whose shell went and which
// launchd took (its parent 1), still working in a worktree's `target/` an
// hour on. One held a core for 28 hours, unseen, every timing on the
// machine since taken beside it (2026-11-09). `check:fast` and `bun run task
// start` say so, and a machine that has one isn't quiet: the ledger's
// numbers are taken only where there's none (docs/dev-loop.md).
//
//   bun scripts/strays.ts     each, by its PID, or nothing
import { readlinkSync } from "node:fs";

export type Stray = { pid: number; seconds: number; cwd: string; command: string };

/** `ps`'s elapsed time, `[[dd-]hh:]mm:ss`, in seconds. */
export function elapsedSeconds(etime: string): number {
  const [days, rest] = etime.includes("-") ? etime.split("-") : ["0", etime];
  const parts = rest.split(":").map(Number);
  while (parts.length < 3) parts.unshift(0);
  const [h, m, s] = parts;
  return ((Number(days) * 24 + h) * 60 + m) * 60 + s;
}

/** Every worktree of the repository `cwd` is in, as git lists them: the
 * main checkout, and each other wherever it is, inside the checkout or
 * beside it (`git worktree add ../x`, #1449). */
export function worktreeRoots(cwd: string): string[] {
  const listed = Bun.spawnSync(["git", "worktree", "list", "--porcelain"], { cwd }).stdout.toString();
  return listed
    .split("\n")
    .filter((line) => line.startsWith("worktree "))
    .map((line) => line.slice("worktree ".length));
}

/** Each process's working directory, of the user's processes: `lsof`'s on
 * macOS, `/proc`'s on Linux. */
function workingDirectories(pids: number[]): Map<number, string> {
  const cwds = new Map<number, string>();
  if (process.platform === "linux") {
    for (const pid of pids) {
      try {
        cwds.set(pid, readlinkSync(`/proc/${pid}/cwd`));
      } catch {}
    }
    return cwds;
  }
  if (pids.length === 0) return cwds;
  const said = Bun.spawnSync(["lsof", "-a", "-d", "cwd", "-p", pids.join(","), "-Fpn"]).stdout.toString();
  let pid = 0;
  for (const line of said.split("\n")) {
    if (line.startsWith("p")) pid = Number(line.slice(1));
    else if (line.startsWith("n") && pid) cwds.set(pid, line.slice(1));
  }
  return cwds;
}

/** The processes parented by 1, running past `minimum` seconds, whose
 * working directory is in a `target/` of a worktree of the repository
 * `cwd` is in, its main checkout's or another's: what the loop left
 * behind. */
export function strays(cwd = process.cwd(), minimum = 60 * 60): Stray[] {
  const roots = worktreeRoots(cwd);
  const listed = Bun.spawnSync(["ps", "-Ao", "pid=,ppid=,etime=,command="]).stdout.toString();
  const candidates: Omit<Stray, "cwd">[] = [];
  for (const line of listed.split("\n")) {
    const match = line.trim().match(/^(\d+)\s+(\d+)\s+(\S+)\s+(.*)$/);
    if (!match || match[2] !== "1") continue;
    const seconds = elapsedSeconds(match[3]);
    if (seconds >= minimum) candidates.push({ pid: Number(match[1]), seconds, command: match[4] });
  }
  const cwds = workingDirectories(candidates.map((c) => c.pid));
  const inTarget = (dir: string) =>
    roots.some((root) => dir.startsWith(`${root}/`) && /(^|\/)target(\/|$)/.test(dir.slice(root.length)));
  return candidates.flatMap((c) => {
    const dir = cwds.get(c.pid);
    return dir !== undefined && inTarget(dir) ? [{ ...c, cwd: dir }] : [];
  });
}

/** What to say of `found`: nothing, or a warning naming each, how long
 * it's run and where, and that the machine isn't quiet. */
export function said(found: Stray[]): string | undefined {
  if (found.length === 0) return undefined;
  const hours = (s: number) => `${(s / 3600).toFixed(1)} h`;
  return [
    `${found.length} process${found.length === 1 ? "" : "es"} the loop left running, parented by launchd, each in a worktree's target/: the machine isn't quiet, and no timing here is a measurement (docs/dev-loop.md)`,
    ...found.map((s) => `  ${s.pid}  ${hours(s.seconds)}  ${s.cwd}  ${s.command.slice(0, 100)}`),
    `stop each that's yours: kill ${found.map((s) => s.pid).join(" ")}`,
  ].join("\n");
}

if (import.meta.main) {
  const warning = said(strays());
  console.log(warning ?? "no process the loop left running");
}
