// A net's failure, given with its program reduced (#1088): each net, the
// metamorphic net's and the neighbour shuffle's, hands what goes wrong to
// `scripts/reduce.ts`, so the failure it reports comes with a small program
// that fails the same way, where an agent reduced each by hand.
//
//   NET_REDUCE=0          the command to reduce it, not reduced
//   NET_REDUCE_BUDGET=<s> how long each may take, 300 s unless it's given
import { spawnSync } from "node:child_process";
import { availableParallelism } from "node:os";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";

import { root, target } from "./support";

/** `file`'s program, which goes wrong in `build`, reduced: what to append
 * to the failure, the reduced program or why there's none, or with
 * `NET_REDUCE=0` the command that reduces it. Of a part of a net run in
 * parts, `RUST_JS_CASES`'s, its share of the cores. */
export function reducedRepro(file: string, build: string, edition = "2024"): string {
  // Kept where the run's fixtures aren't, which go as it ends.
  const kept = join(target, "reduce", "net", `${Bun.hash(readFileSync(file, "utf8")).toString(36)}.rs`);
  mkdirSync(join(kept, ".."), { recursive: true });
  copyFileSync(file, kept);
  const command = `bun scripts/reduce.ts ${relative(root, kept)} --build ${build} --edition ${edition}`;
  if (process.env.NET_REDUCE === "0") return `\nreduce it: ${command}`;
  const parts = Number((process.env.RUST_JS_CASES ?? "1/1").split("/")[1]) || 1;
  const jobs = Math.max(1, Math.floor(availableParallelism() / parts));
  const budget = process.env.NET_REDUCE_BUDGET ?? "300";
  const out = kept.replace(/\.rs$/, `.${build}.reduced.rs`);
  const p = spawnSync(
    "bun",
    ["scripts/reduce.ts", kept, "--build", build, "--edition", edition, "--budget", budget, "--jobs", String(jobs), "--out", out],
    { cwd: root, encoding: "utf8", timeout: (Number(budget) + 300) * 1000, maxBuffer: 64 * 1024 * 1024 },
  );
  const said = (p.stderr ?? "").trim().split("\n").pop() ?? "";
  if (p.status !== 0) return `\nnot reduced (${said || p.error?.message || "the reducer failed"}): ${command}`;
  return `\n${said}:\n${p.stdout.trimEnd()}`;
}
