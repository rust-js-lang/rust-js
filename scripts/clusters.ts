// The kind × area count (ADR 1085, docs/bug-kinds.md's "Reading the
// signal"): every issue's `kind: …` and `area: …` labels, each cell's open
// and closed issues over a window and all time, and each cell of 3 or more
// a cluster, which calls for the refactor its kind names in
// docs/bug-kinds.md's table, the one authority, read from it here.
//
//   bun scripts/clusters.ts                   the count, the last 14 days and all time
//   bun scripts/clusters.ts --days=30         over another window
//   bun scripts/clusters.ts --comment=1681    and the open clusters, one comment on that issue, edited in place (the nightly's)
//
// Issues by `nguyenyou` and `github-actions` alone: anyone else's may be
// spam, and isn't counted. A cluster's tracking refactor is an open issue
// whose body has a line `cluster: <kind> × <area>`.

import { readFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");

/** The authors whose issues count. */
export const authors = ["nguyenyou", "github-actions", "app/github-actions"];

/** A cluster is this many issues or more of one kind in one area. */
export const clusterSize = 3;

/** Each kind's label and what a cluster of it calls for, from
 * docs/bug-kinds.md's table of kinds: its `Label` and `A cluster calls for`
 * columns. */
export function kindsTable(page: string): Map<string, string> {
  const lines = page.split("\n");
  const header = lines.findIndex((line) => line.startsWith("|") && line.includes("| Label |") && line.includes("| A cluster calls for |"));
  if (header === -1) throw new Error("docs/bug-kinds.md has no table of kinds with `Label` and `A cluster calls for` columns");
  const cells = (line: string) => line.split("|").slice(1, -1).map((cell) => cell.trim());
  const columns = cells(lines[header]!);
  const label = columns.indexOf("Label");
  const calls = columns.indexOf("A cluster calls for");
  const kinds = new Map<string, string>();
  for (const line of lines.slice(header + 2)) {
    if (!line.startsWith("|")) break;
    const row = cells(line);
    kinds.set(row[label]!.replace(/`/g, ""), row[calls]!);
  }
  return kinds;
}

/** An issue as `gh issue list --json` gives it. */
export type Issue = { number: number; state: string; createdAt: string; author: { login: string }; labels: { name: string }[] };

/** A cell of the count: its kind and area, and its issues, all time and
 * over the window, each open or closed. */
export type Cell = { kind: string; area: string; open: number[]; closed: number[]; recentOpen: number[]; recentClosed: number[] };

/** Each kind × area cell of `issues`, counted where they're by `authors`,
 * the window those created since `since`; the largest first. An issue
 * without both labels isn't counted. */
export function count(issues: Issue[], since: Date): Cell[] {
  const cells = new Map<string, Cell>();
  for (const issue of issues) {
    if (!authors.includes(issue.author.login)) continue;
    const names = issue.labels.map(({ name }) => name);
    const kind = names.find((name) => name.startsWith("kind: "));
    const area = names.find((name) => name.startsWith("area: "));
    if (kind === undefined || area === undefined) continue;
    const key = `${kind}\0${area}`;
    const cell = cells.get(key) ?? { kind, area, open: [], closed: [], recentOpen: [], recentClosed: [] };
    const open = issue.state.toLowerCase() === "open";
    (open ? cell.open : cell.closed).push(issue.number);
    if (new Date(issue.createdAt) >= since) (open ? cell.recentOpen : cell.recentClosed).push(issue.number);
    cells.set(key, cell);
  }
  const size = (cell: Cell) => cell.open.length + cell.closed.length;
  return [...cells.values()].sort((a, b) => size(b) - size(a) || a.area.localeCompare(b.area) || a.kind.localeCompare(b.kind));
}

/** Whether a cell is a cluster, all time or over the window. */
export const isCluster = (cell: Cell, recent = false): boolean =>
  (recent ? cell.recentOpen.length + cell.recentClosed.length : cell.open.length + cell.closed.length) >= clusterSize;

/** The cell's name in a tracking issue's body: `cluster: <kind> × <area>`,
 * the labels without their prefixes. */
export const clusterLine = (cell: { kind: string; area: string }) => `cluster: ${cell.kind.replace(/^kind: /, "")} × ${cell.area.replace(/^area: /, "")}`;

/** The open issue tracking each cluster's refactor, by `clusterLine`. */
export function trackers(cells: Cell[], open: { number: number; body: string }[]): Map<Cell, number> {
  const found = new Map<Cell, number>();
  for (const cell of cells) {
    const line = clusterLine(cell).toLowerCase();
    const tracker = open.find(({ body }) => body.split("\n").some((text) => text.trim().toLowerCase() === line));
    if (tracker) found.set(cell, tracker.number);
  }
  return found;
}

/** The count, a table of each cell, a cluster marked, and its refactor. */
export function table(cells: Cell[], kinds: Map<string, string>, days: number): string {
  const rows = cells.map((cell) => {
    const all = cell.open.length + cell.closed.length;
    const recent = cell.recentOpen.length + cell.recentClosed.length;
    const mark = isCluster(cell) || isCluster(cell, true) ? "cluster" : "";
    return `| ${cell.area} | ${cell.kind} | ${recent} (${cell.recentOpen.length} open) | ${all} (${cell.open.length} open) | ${mark} | ${mark ? (kinds.get(cell.kind) ?? "no refactor named: the kind isn't docs/bug-kinds.md's") : ""} |`;
  });
  return [`| Area | Kind | Last ${days} days | All time | | A cluster calls for |`, "|---|---|---|---|---|---|", ...rows].join("\n");
}

/** The comment on the campaign: each open cluster, a line, its issues and
 * its tracking refactor's or that there's none. */
export function comment(cells: Cell[], tracked: Map<Cell, number>, days: number, at: string): string {
  const open = cells.filter((cell) => (isCluster(cell) || isCluster(cell, true)) && cell.open.length > 0);
  return [
    marker,
    `Open kind × area clusters, ${at} (\`bun scripts/clusters.ts\`, ADR 1085; ${clusterSize} or more issues of one kind in one area, all time or the last ${days} days, with an issue open):`,
    "",
    ...(open.length === 0
      ? ["None."]
      : open.map((cell) => {
          const tracker = tracked.get(cell);
          const issues = cell.open.map((n) => `#${n}`).join(" ");
          return `- **${cell.area} × ${cell.kind}**: ${cell.open.length} open, ${cell.closed.length} closed (${issues}); ${tracker ? `refactor tracked in #${tracker}` : `no tracking refactor (an open issue with a \`${clusterLine(cell)}\` line)`}`;
        })),
  ].join("\n");
}

/** What marks the comment the nightly edits in place. */
export const marker = "<!-- rust-js clusters -->";

function gh(args: string[]): string {
  const p = Bun.spawnSync(["gh", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`gh ${args.join(" ")} failed: ${p.stderr.toString()}`);
  return p.stdout.toString();
}

if (import.meta.main) {
  const arg = (name: string) => process.argv.find((given) => given.startsWith(`--${name}=`))?.slice(name.length + 3);
  const days = Number(arg("days") ?? 14);
  const issues: Issue[] = JSON.parse(gh(["issue", "list", "--state=all", "--limit=5000", "--json=number,state,createdAt,author,labels"]));
  const cells = count(issues, new Date(Date.now() - days * 86_400_000));
  const kinds = kindsTable(readFileSync(join(root, "docs", "bug-kinds.md"), "utf8"));
  console.log(table(cells, kinds, days));
  const campaign = arg("comment");
  if (campaign) {
    const open: { number: number; body: string; author: { login: string } }[] = JSON.parse(gh(["issue", "list", "--state=open", "--limit=1000", "--search=cluster in:body", "--json=number,body,author"]));
    const tracked = trackers(
      cells,
      open.filter((issue) => authors.includes(issue.author.login)),
    );
    const body = comment(cells, tracked, days, new Date().toISOString().slice(0, 10));
    const comments: { id: number; body: string; login: string }[] = gh(["api", `repos/{owner}/{repo}/issues/${campaign}/comments?per_page=100`, "--paginate", "--jq", ".[] | {id, body, login: .user.login}"]).split("\n").filter(Boolean).map((line) => JSON.parse(line));
    const mine = comments.find((c) => c.body.startsWith(marker) && authors.some((login) => c.login === login || `${c.login}[bot]` === login || c.login === `${login}[bot]`));
    if (mine) gh(["api", "--method=PATCH", `repos/{owner}/{repo}/issues/comments/${mine.id}`, "-f", `body=${body}`]);
    else gh(["issue", "comment", campaign, "--body", body]);
    console.log(`\n${mine ? "edited" : "wrote"} the clusters' comment on #${campaign}`);
  }
}
