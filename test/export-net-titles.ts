// The export net's tests (#2481), each by its title and its part of
// `exportNetParts` (`partOf` of its title): each own case of
// `test/export-net/`, the offered paths of #2544's shapes pinned, and each
// crate pair of `test/crates/pairs/`. The one list: test/export-net-parts.ts
// registers each in its part's file, and test/test-names.ts lists them,
// which a read of the files' literals can't. No `bun:test` here, so
// scripts read it too.
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { exportNetParts, partOf } from "../scripts/shard";
import { root } from "./support";

export const ownCases = join(root, "test", "export-net");
export const pairCases = join(root, "test", "crates", "pairs");

export type NetTest = { title: string; part: number } & (
  | { kind: "own" | "pair"; cases: string; name: string; files: string[] }
  | { kind: "pinned" }
);

/** Each of the export net's tests, in the order each part registers its own. */
export function exportNetTitles(): NetTest[] {
  const of = (title: string) => ({ title, part: partOf(title, exportNetParts) });
  return [
    ...readdirSync(ownCases).sort().map((name) => ({
      ...of(`export net: ${name}`), kind: "own" as const, cases: ownCases, name,
      files: readdirSync(join(ownCases, name)).includes("lib.rs") ? ["lib"] : ["dep", "app"],
    })),
    { ...of("the paths a crate offers JS are its root's and each re-exported module's, not a pub mod's own"), kind: "pinned" as const },
    ...readdirSync(pairCases).sort().map((name) => ({ ...of(`export net: two crates: ${name}`), kind: "pair" as const, cases: pairCases, name, files: ["dep", "app"] })),
  ];
}
