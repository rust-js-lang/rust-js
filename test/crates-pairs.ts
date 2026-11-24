// The crate pairs' table (`test/crates/pairs/<case>/`): a library, `dep`, and
// an `app` using it, for each thing crossing between them that a consumer must
// be told, or must assume, each run as native Rust does. Found in review: each
// had printed another answer. In `cratePairParts` files, each the pairs a hash
// of the name puts in its part (`partOf`), so the pool runs them at once
// (#2465).
import { test } from "bun:test";
import { readdirSync } from "node:fs";
import { cratePairParts, partOf } from "../scripts/shard";
import { chain, pairs } from "./crates";

/** A test of each pair in `part`, from 1, of `cratePairParts`. */
export function pairTests(part: number) {
  for (const name of readdirSync(pairs).sort()) {
    if (partOf(name, cratePairParts) !== part) continue;
    test(`two crates: ${name}`, () => {
      chain(pairs, name, ["dep", "app"]);
    }, 300_000);
  }
}
