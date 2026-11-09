// A push that undoes another's commit is refused (`.githooks/pre-push`,
// which `bun install` sets git to run). A commit made from a tree that was
// older than the main it's pushed onto, as `git reset --soft origin/main`
// after a fetch makes one, carries each file the newer commits changed back
// as it was before them: 4163f3865 took 12e759c57's P0 fix off `main` so,
// unseen till its author looked (2026-11-09). Each file a push changes is
// held to its own history on the remote: where what's pushed is that file
// as it was before one of its last commits there, of the last day, the push
// undoes that commit, and is refused, unless a commit of the push names it,
// as `git revert`'s "This reverts commit <sha>" does.
//
//   git push                       the hook runs it, given git's lines on stdin
//   bun scripts/push-guard.ts <remote-sha> <local-sha>
import { spawnSync } from "node:child_process";

/** A file of a push, and the commit of the remote's it would undo. */
export type Undone = { file: string; commit: string; subject: string };

const day = 24 * 60 * 60;

/** `git` in `cwd`, its stdout, trimmed; `undefined` where it fails. */
function git(cwd: string, args: string[]): string | undefined {
  const p = spawnSync("git", args, { cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  return p.status === 0 ? p.stdout.trim() : undefined;
}

/** The blob `file` is at `commit`, or `-` where it isn't there. */
const blob = (cwd: string, commit: string, file: string) => git(cwd, ["rev-parse", "--quiet", "--verify", `${commit}:${file}`]) ?? "-";

/** What pushing `local` onto `remote` would undo: each file it changes
 * that's as it was before one of the remote's commits of `file`, the last
 * `recent` of them of the last day before `now` (seconds), which no commit
 * of the push names. */
export function undone(cwd: string, remote: string, local: string, { now = Date.now() / 1000, recent = 20 } = {}): Undone[] {
  const changed = (git(cwd, ["diff", "--name-only", "--no-renames", remote, local]) ?? "").split("\n").filter(Boolean);
  const messages = git(cwd, ["log", "--format=%B", `${remote}..${local}`]) ?? "";
  const named = (commit: string) => messages.includes(commit.slice(0, 9));
  const found: Undone[] = [];
  for (const file of changed) {
    const pushed = blob(cwd, local, file);
    const history = (git(cwd, ["log", `-n${recent}`, "--format=%H %ct %s", remote, "--", file]) ?? "").split("\n").filter(Boolean);
    for (const line of history) {
      const [commit, time, ...words] = line.split(" ");
      if (now - Number(time) > day) break;
      const before = blob(cwd, `${commit}^`, file);
      if (before === pushed && blob(cwd, commit, file) !== pushed && !named(commit)) {
        found.push({ file, commit, subject: words.join(" ") });
        break;
      }
    }
  }
  return found;
}

/** What to say of `found`: nothing, or the refusal, each file and what it
 * undoes, and what to do. */
export function refusal(found: Undone[]): string | undefined {
  if (found.length === 0) return undefined;
  const commits = [...new Set(found.map((u) => u.commit))];
  return [
    "push refused: it undoes commits already on the remote, as a commit made from an older tree does (scripts/push-guard.ts):",
    ...found.map((u) => `  ${u.file}: back to before ${u.commit.slice(0, 9)} ${u.subject.slice(0, 80)}`),
    `rebase onto the remote and commit again (\`git rebase origin/main\`); or, to revert ${commits.map((c) => c.slice(0, 9)).join(", ")} on purpose, name it in the message ("This reverts commit <sha>")`,
  ].join("\n");
}

if (import.meta.main) {
  const zero = /^0+$/;
  // The hook's lines, `<local ref> <local sha> <remote ref> <remote sha>`,
  // or a remote and a local commit given.
  const pairs: [string, string][] =
    process.argv.length >= 4
      ? [[process.argv[2], process.argv[3]]]
      : (await Bun.stdin.text())
          .split("\n")
          .filter(Boolean)
          .map((line) => line.split(" "))
          .filter(([, local, , remote]) => !zero.test(local) && !zero.test(remote))
          .map(([, local, , remote]): [string, string] => [remote, local]);
  const found = pairs.flatMap(([remote, local]) => undone(process.cwd(), remote, local));
  const said = refusal(found);
  if (said) {
    console.error(said);
    process.exit(1);
  }
}
