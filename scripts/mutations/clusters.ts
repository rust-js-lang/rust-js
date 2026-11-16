// The issues' kind × area count (scripts/clusters.ts, ADR 1085, amended):
// a count that trusts anyone's issues, or calls fewer a cluster, picks the
// wrong refactor.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "clusters-anyones-issues",
    breaks: "issues by anyone are counted, though the project acts on nguyenyou's and github-actions' alone",
    file: "scripts/clusters.ts",
    find: "    if (!authors.includes(issue.author.login)) continue;",
    replace: "",
    tests: ["test/clusters.test.ts", "-t", "each kind × area cell counts"],
  },
  {
    name: "clusters-of-two",
    breaks: "two issues of one kind in one area are called a cluster",
    file: "scripts/clusters.ts",
    find: "export const clusterSize = 3;",
    replace: "export const clusterSize = 2;",
    tests: ["test/clusters.test.ts", "-t", "each kind × area cell counts"],
  },
  {
    name: "clusters-closed-listed-open",
    breaks: "a cluster whose every issue is closed is listed on the campaign as open",
    file: "scripts/clusters.ts",
    find: "(isCluster(cell) || isCluster(cell, true)) && cell.open.length > 0);",
    replace: "(isCluster(cell) || isCluster(cell, true)));",
    tests: ["test/clusters.test.ts", "-t", "the campaign's comment"],
  },
];
