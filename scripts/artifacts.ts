// A workflow run's artifacts, downloaded: the one way every script fetches
// them (`gh run download`), each retry into an empty directory.
import { mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");

/** `run`'s artifacts `select` names (`--name=..` or `--pattern=..`) into
 * `dir`, by `fetch` (gh's), asked three times, each into `dir` emptied
 * first: a retry into what a failed one left half extracted failed again,
 * `openat timings.json: file exists`, and the shard with it (check of
 * 6d9fcfb45, a 503 "Egress is over the account limit" first). One that
 * matches none is asked once. Of `seconds`, each ask ends past them, and
 * one that did isn't asked again: GitHub's storage served the push's 64 MB
 * map at some 85 KB/s, 12 min a push waited on it (#2385), and an ask
 * again is as slow. Whether it got them. */
export function downloaded(
  run: number | string,
  select: string[],
  dir: string,
  fetch: (args: string[], seconds?: number) => { ok: boolean; said: string; timedOut?: boolean } = (args, seconds) => {
    const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe", timeout: seconds === undefined ? undefined : seconds * 1000 });
    const timedOut = p.exitedDueToTimeout === true;
    return { ok: p.exitCode === 0, said: timedOut ? `no answer in ${seconds} s` : p.stderr.toString(), timedOut };
  },
  wait: (ms: number) => void = Bun.sleepSync,
  seconds?: number,
): boolean {
  for (let attempt = 1; attempt <= 3; attempt++) {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    const got = fetch(["run", "download", String(run), ...select, `--dir=${dir}`], seconds);
    if (got.ok) return true;
    const none = /no valid artifacts found|no artifact matches/i.test(got.said);
    const once = none || got.timedOut === true;
    console.error(`gh run download ${run} ${select.join(" ")} failed:\n${got.said.trimEnd()}${once || attempt === 3 ? "" : `\nasking again in ${10 * attempt} s`}`);
    if (once) return false;
    if (attempt < 3) wait(10000 * attempt);
  }
  return false;
}
