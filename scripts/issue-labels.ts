// Every open issue carries the labels docs/bug-kinds.md asks for (ADR 1085):
// exactly one priority (CONTRIBUTING.md's `P0`..`P3`), or `tracking` for a
// tracker, or a `perf: …` label for a performance issue; and, unless it's a tracker or a dev-loop issue, one `kind: …` and
// one `area: …`. A `Nightly red: <date>` report carries none of these.
//
// `bun scripts/issue-labels.ts`             lists the open issues that miss a label
// `bun scripts/issue-labels.ts --issue 12`  checks one issue
// `bun scripts/issue-labels.ts --mark`      also labels each one `needs-labels`, says
//                                           once what it misses, and unlabels one that's fixed
// The `issue labels` workflow runs it on each issue opened, edited or
// relabelled, and daily over every open issue.

type Issue = { number: number; title: string; labels: { name: string }[]; author: { login: string } };

const args = process.argv.slice(2);
const mark = args.includes("--mark");
const one = args.includes("--issue") ? Number(args[args.indexOf("--issue") + 1]) : undefined;
const marker = "<!-- issue-labels -->";

function gh(...argv: string[]): string {
  const run = Bun.spawnSync(["gh", ...argv], { stdout: "pipe", stderr: "pipe" });
  if (run.exitCode !== 0) throw new Error(`gh ${argv.join(" ")}: ${run.stderr.toString()}`);
  return run.stdout.toString();
}

/** What `issue` misses, by group; empty where it has all it needs. */
export function missing(issue: Issue): string[] {
  if (issue.title.startsWith("Nightly red:")) return [];
  const names = issue.labels.map((label) => label.name);
  const priorities = names.filter((name) => /^P[0-3]: /.test(name));
  const tracker = names.includes("tracking");
  // A performance issue changes no output, so it ranks by its perf label (`perf: runtime`, `perf: compile`).
  const perf = names.some((name) => name.startsWith("perf: "));
  const needs: string[] = [];
  if (!tracker && priorities.length !== 1 && !(perf && priorities.length === 0)) {
    needs.push(priorities.length === 0 ? "a priority (`P0: wrong answer` … `P3: readability`), or `tracking`" : `exactly one priority (it has ${priorities.join(", ")})`);
  }
  if (!tracker && !names.includes("dev-loop")) {
    if (!names.some((name) => name.startsWith("kind: "))) needs.push("a `kind: …`");
    if (!names.some((name) => name.startsWith("area: "))) needs.push("an `area: …`");
  }
  return needs;
}

if (import.meta.main) {
  const fields = "number,title,labels,author";
  const issues: Issue[] =
    one === undefined
      ? JSON.parse(gh("issue", "list", "--state", "open", "--limit", "1000", "--json", fields))
      : [JSON.parse(gh("issue", "view", String(one), "--json", fields))];
  let short = 0;
  for (const issue of issues) {
    const needs = missing(issue);
    const flagged = issue.labels.some((label) => label.name === "needs-labels");
    if (needs.length > 0) {
      short++;
      console.log(`#${issue.number} needs ${needs.join(", ")}: ${issue.title}`);
      if (mark && !flagged) {
        gh("issue", "edit", String(issue.number), "--add-label", "needs-labels");
        const said = gh("issue", "view", String(issue.number), "--json", "comments", "--jq", `[.comments[].body | select(contains("${marker}"))] | length`).trim();
        if (said === "0") {
          gh("issue", "comment", String(issue.number), "--body", `${marker}\nThis issue needs ${needs.join(", ")}, as [docs/bug-kinds.md](https://github.com/rust-js-lang/rust-js/blob/main/docs/bug-kinds.md) says. The \`needs-labels\` label comes off when it has them.`);
        }
      }
    } else if (mark && flagged) {
      gh("issue", "edit", String(issue.number), "--remove-label", "needs-labels");
    }
  }
  if (one === undefined) console.log(short === 0 ? `All ${issues.length} open issues are labelled.` : `${short} of ${issues.length} open issues miss a label.`);
  process.exit(short > 0 && !mark ? 1 : 0);
}
