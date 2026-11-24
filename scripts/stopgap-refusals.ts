// Campaign 6's stopgap refusals (#2539, ADR 1201), read from their one
// owner: `refusal::Crossing`, each cell of the matrix a refusal names when
// it's there only because a crossing doesn't convert a form or a fact yet,
// and `Crossing::lifted_by`, the open issue that lifts each.
//
// test/architecture-stopgaps.test.ts holds each cell refused somewhere and
// mapped once; run, this checks each cell's issue is open, as the
// issue-labels workflow does when an issue closes and daily: an issue
// closed while its cell's refusals remain fails, as the campaign would
// have lost track of them.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

export const owner = "crates/rust-js-lower/src/refusal.rs";

export type Cell = { crossing: string; issue: number };

/** Each `Crossing` and the issue `lifted_by` maps it to, of refusal.rs's text. */
export function cells(source: string): Cell[] {
  const body = source.slice(source.indexOf("pub fn lifted_by"));
  return [...body.slice(0, body.indexOf("\n    }\n")).matchAll(/Crossing::(\w+) => (\d+),/g)].map((m) => ({
    crossing: m[1],
    issue: Number(m[2]),
  }));
}

/** The cells whose issue GitHub says is closed. */
export function closed(listed: Cell[], state: (issue: number) => string): Cell[] {
  return listed.filter(({ issue }) => state(issue) !== "OPEN");
}

if (import.meta.main) {
  // `--mark`, the workflow's: each closed one is told so on its issue, once.
  const mark = process.argv.includes("--mark");
  const gh = (...args: string[]) => {
    const run = spawnSync("gh", args, { encoding: "utf8" });
    if (run.status !== 0) throw new Error(`gh ${args.join(" ")}: ${run.stderr}`);
    return run.stdout.trim();
  };
  const listed = cells(readFileSync(join(import.meta.dir, "..", owner), "utf8"));
  if (listed.length === 0) throw new Error(`no \`Crossing::lifted_by\` arms read in ${owner}`);
  const shut = closed(listed, (issue) => gh("issue", "view", String(issue), "--json", "state", "--jq", ".state"));
  if (shut.length > 0) {
    const named = shut.map(({ crossing, issue }) => `#${issue} (\`Crossing::${crossing}\`)`).join(", ");
    console.error(`Closed while their stopgap refusals remain (${owner}): ${named}. Reopen each, or lift its refusals and remove its cell.`);
    if (mark) {
      const marker = "<!-- stopgap-refusals -->";
      for (const { crossing, issue } of shut) {
        const said = gh("issue", "view", String(issue), "--json", "comments", "--jq", `[.comments[].body | select(contains("${marker}"))] | length`);
        if (said !== "0") continue;
        gh("issue", "comment", String(issue), "--body", `${marker}\nThis issue lifts Campaign 6's stopgap cell \`Crossing::${crossing}\` (#2539, ADR 1201), whose refusals remain on \`main\`: \`git grep -n "Crossing::${crossing}\\b" crates\`. Reopen it, or lift them and remove the cell from \`${owner}\`.`);
      }
    }
    process.exit(1);
  }
  console.log(`All ${listed.length} stopgap cells' issues are open.`);
}
