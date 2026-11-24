// A list of what to run, given to a runner by a person or a selector: the
// corpus's cases (`RUST_JS_CASES=i/n:<file>`, which the nets and stages
// that run the corpus's cases read too), the metamorphic net's rewrites,
// the crate corpus's and the Crater's crates, rustc's tests' folders, the
// nightly's jobs and the mutations' names. A list naming what the runner
// has none of ran nothing of it and passed: the metamorphic net given its
// cases as `test/corpus/...` paths, where it takes their names, ran none
// and said "25 pass" (2026-11-25). The one check every runner asks.

import { basename } from "node:path";

/** Of `named`, each entry `known` has nothing of: a set of the names a
 * runner can run, or whether an entry matches any, of a selector that's a
 * prefix. */
export function unmatchedOf(named: string[], known: ReadonlySet<string> | ((entry: string) => boolean)): string[] {
  const matches = matcher(known);
  return [...new Set(named.filter((entry) => !matches(entry)))];
}

const matcher = (known: ReadonlySet<string> | ((entry: string) => boolean)) => (typeof known === "function" ? known : (entry: string) => known.has(entry));

/** Refuse a list whose entries aren't each something its runner can run:
 * one that matches nothing, or fewer than it names, with every entry
 * unmatched named and, of one given as a path where its file's name is
 * one, the name to give. `list` says the list as it was given, `what` what
 * its entries are. An empty list names nothing, and each runner says what
 * it runs of one. */
export function refuseUnmatched(named: string[], known: ReadonlySet<string> | ((entry: string) => boolean), { list, what }: { list: string; what: string }) {
  const unmatched = unmatchedOf(named, known);
  if (unmatched.length === 0) return;
  const pathed = unmatched.find((entry) => entry.includes("/") && matcher(known)(basename(entry)));
  const asName = pathed === undefined ? "" : `; ${what} are named by their names, \`${basename(pathed)}\`, not by a path, \`${pathed}\``;
  const none = unmatched.length === new Set(named).size ? ", so it would run none of them" : "";
  throw new Error(`${list} names ${unmatched.length} of its ${new Set(named).size} that aren't ${what}${none}: ${unmatched.join(", ")}${asName}`);
}
