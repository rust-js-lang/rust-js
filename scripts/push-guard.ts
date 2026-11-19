// A push that undoes another's commit is refused (`.githooks/pre-push`,
// which `bun install` sets git to run). A commit made from a tree that was
// older than the main it's pushed onto, as `git reset --soft origin/main`
// after a fetch makes one, carries each file the newer commits changed back
// as it was before them: 4163f3865 took 12e759c57's P0 fix off `main` so,
// unseen till its author looked (2026-11-09). Each file a push changes is
// held to its own history on the remote: where what's pushed is that file
// as it was before one of its last commits there, of the last day, the push
// undoes that commit, and is refused, unless a commit of the push names it,
// as `git revert`'s "This reverts commit <sha>" does. And where a commit's
// whole change, each line it added and took away in each file, applies to
// what's pushed again, where it didn't to the remote, the push undoes it
// too: f7d5e10ec took 02df83a40's memory cap out of four files so, each
// file another's edit too, never one of them as it was (#1097). But rows
// a push takes off a known list, `test/*-known.txt`, each under the heading
// of an issue one of its commits `Fixes`, are what fixing that issue is,
// not an undoing of the commit that listed them: #2146's, #2139's and one
// before were refused so, and their authors had to name that commit.
// `bun run push` runs it first, its `fast` phase, as well as git's hook.
//
//   git push                       the hook runs it, given git's lines on stdin
//   bun scripts/push-guard.ts <remote-sha> <local-sha>
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/** A file of a push, and the commit of the remote's it would undo. */
export type Undone = { file: string; commit: string; subject: string };

const day = 24 * 60 * 60;

/** `git` in `cwd`, its stdout, trimmed; `undefined` where it fails. */
function git(cwd: string, args: string[], env: Record<string, string> = {}, input?: string): string | undefined {
  const p = spawnSync("git", args, { cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, env: { ...process.env, ...env }, input });
  return p.status === 0 ? p.stdout.trim() : undefined;
}

/** Whether `patch` applies to the tree of `commit`, in an index of its own,
 * with all its context: one's own edit within three lines of another's
 * change isn't read as undoing it, as `-C1` read the removal of a refusal's
 * test case once it compiled (34ab92bb2's) beside others' (2026-11-09). */
function appliesTo(cwd: string, commit: string, patch: string, index: string, reverse = false): boolean {
  const env = { GIT_INDEX_FILE: index };
  return git(cwd, ["read-tree", commit], env) !== undefined && git(cwd, ["apply", "--cached", "--check", ...(reverse ? ["-R"] : []), "-"], env, patch) !== undefined;
}

/** The blob `file` is at `commit`, or `-` where it isn't there. */
const blob = (cwd: string, commit: string, file: string) => git(cwd, ["rev-parse", "--quiet", "--verify", `${commit}:${file}`]) ?? "-";

/** What pushing `local` onto `remote` would undo: each file it changes
 * that's as it was before one of the remote's commits of `file`, the last
 * `recent` of them of the last day before `now` (seconds); and each file of
 * one of the remote's last `recent` commits of the files it changes, of the
 * last day, whose whole change applies to `local` again where it didn't to
 * `remote`; each of a commit no commit of the push names. */
export function undone(cwd: string, remote: string, local: string, { now = Date.now() / 1000, recent = 20 } = {}): Undone[] {
  const messages = git(cwd, ["log", "--format=%B", `${remote}..${local}`]) ?? "";
  const fixes = fixedIssues(messages);
  const changed = (git(cwd, ["diff", "--name-only", "--no-renames", remote, local]) ?? "")
    .split("\n")
    .filter(Boolean)
    .filter((file) => !fixedRowsOnly(file, git(cwd, ["show", `${remote}:${file}`]) ?? "", git(cwd, ["diff", "-U0", "--no-renames", remote, local, "--", file]) ?? "", fixes));
  const named = (commit: string) => messages.includes(commit.slice(0, 9));
  const found: Undone[] = [];
  // Each recent commit of the remote's of a file it changes whose whole
  // change applies to what's pushed again, and didn't to the remote.
  const scratch = mkdtempSync(join(tmpdir(), "push-guard-"));
  try {
    const since = `--since=@${Math.floor(now - day)}`;
    // None where it changes none but fixed rows: `git log --` of no file is of every one.
    const recentCommits = changed.length === 0 ? [] : (git(cwd, ["log", `-n${recent}`, since, "--format=%H %s", remote, "--", ...changed]) ?? "").split("\n").filter(Boolean);
    for (const line of recentCommits) {
      const [commit, ...words] = line.split(" ");
      if (named(commit)) continue;
      const patch = git(cwd, ["diff", "--binary", "--no-renames", `${commit}^`, commit]);
      if (!patch) continue;
      const index = join(scratch, commit);
      // And the remote still has it, its change taken off again there: a
      // commit a later one undid, 24e269c11's section that 11084981b took
      // off, doesn't apply to the remote either, where its section was at
      // the file's end and another's follows it now (#2009).
      const has = appliesTo(cwd, remote, `${patch}\n`, `${index}.has`, true);
      if (has && appliesTo(cwd, local, `${patch}\n`, `${index}.local`) && !appliesTo(cwd, remote, `${patch}\n`, `${index}.remote`)) {
        const files = (git(cwd, ["diff", "--name-only", "--no-renames", `${commit}^`, commit]) ?? "").split("\n").filter(Boolean);
        for (const file of files) found.push({ file, commit, subject: words.join(" ") });
      }
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
  for (const file of changed) {
    const pushed = blob(cwd, local, file);
    const history = (git(cwd, ["log", `-n${recent}`, "--format=%H %ct %s", remote, "--", file]) ?? "").split("\n").filter(Boolean);
    // What a later commit of the remote's made the file, newest first: a
    // file as it was before a commit that a later one put back already,
    // as 11084981b took off the section 24e269c11 added, undoes nothing
    // of it (#2009).
    const later = new Set<string>();
    for (const line of history) {
      const [commit, time, ...words] = line.split(" ");
      if (now - Number(time) > day) break;
      const before = blob(cwd, `${commit}^`, file);
      const after = blob(cwd, commit, file);
      if (before === pushed && after !== pushed && !later.has(pushed) && !named(commit)) {
        if (!found.some((u) => u.file === file && u.commit === commit)) found.push({ file, commit, subject: words.join(" ") });
        break;
      }
      later.add(after);
    }
  }
  return found;
}

/** The issues the commits `messages` name: each `#<n>` of a `Fixes ..`
 * or a `Refs ..`, up to its line's end or the parenthesis that closes it.
 * A `Refs` is a partial fix of an issue of many rows, the rows it takes off
 * a known list fixed and the rest left under it (#1892's, 2026-11-19);
 * naming the issue is what makes their removal deliberate. */
export function fixedIssues(messages: string): Set<string> {
  return new Set([...messages.matchAll(/\b(?:Fixes|Refs)\b([^)\n]*)/g)].flatMap((m) => [...m[1]!.matchAll(/#(\d+)/g)].map((n) => n[1]!)));
}

/** Whether `diff`, `git diff -U0` of `file` as the remote has it, `before`,
 * only takes rows off a known list, `test/<name>-known.txt`, each under an
 * issue's heading, `#<n> ..`, of `fixes`, the issues the push names
 * (`fixedIssues`), the heading too where it goes
 * with them: one added, or under another's heading, a `#difference`'s say,
 * isn't. */
export function fixedRowsOnly(file: string, before: string, diff: string, fixes: Set<string>): boolean {
  if (!/^test\/[^/]+-known\.txt$/.test(file)) return false;
  const lines = before.split("\n");
  let removed = 0;
  for (const line of diff.split("\n")) {
    if (line.startsWith("+") && !line.startsWith("+++")) return false;
    const hunk = line.match(/^@@ -(\d+)(?:,(\d+))? \+/);
    if (hunk === null) continue;
    const from = Number(hunk[1]);
    const count = hunk[2] === undefined ? 1 : Number(hunk[2]);
    for (let at = from - 1; at < from - 1 + count; at++) {
      // A blank line between sections goes with either.
      if (lines[at]!.trim() === "") continue;
      let heading = at;
      while (heading >= 0 && !lines[heading]!.startsWith("#")) heading--;
      const issue = heading < 0 ? undefined : lines[heading]!.match(/^#(\d+)\b/)?.[1];
      if (issue === undefined || !fixes.has(issue)) return false;
      removed++;
    }
  }
  return removed > 0;
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
