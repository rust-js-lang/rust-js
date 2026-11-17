// docs/guide/limitations.md's examples, read by the tests that check them
// alone: in test/support.ts, which every test imports, a read of the page
// made each test a reader of it, so the selector ran the whole suite for an
// edit of one line of it (#2003's interaction sanity, 2026-11-17).

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { root } from "./support";

/** Each `- item: \`example\`` line of docs/guide/limitations.md, its item,
 * its example and the name of limitations.test.ts's test that compiles it,
 * which a mutation's `-t` may name. */
export function limitations(): { item: string; example: string; title: string }[] {
  const page = readFileSync(join(root, "docs/guide/limitations.md"), "utf8");
  return [...page.matchAll(/^- (.+?): `([^`]+)`$/gm)].map((m) => ({
    item: m[1],
    example: m[2],
    title: `refused, as docs/guide/limitations.md says: ${m[1]}`,
  }));
}
