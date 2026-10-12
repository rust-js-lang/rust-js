// The built-ins the tests' Bun has are Node's (`node-surface.ts`): Bun's
// own are all taken away, so code that calls one throws here as it would
// in Node and browsers.
import { expect, test } from "bun:test";
import { join } from "node:path";

import { surfaces } from "./node-surface";

const names = () =>
  Object.fromEntries(
    Object.entries(surfaces()).map(([where, at]) => [where, at ? Object.getOwnPropertyNames(at).sort() : []]),
  );

test("the tests' built-ins are Node's, not Bun's own", () => {
  const script = `import { surfaces } from ${JSON.stringify(join(import.meta.dir, "node-surface.ts"))};
console.log(JSON.stringify(Object.fromEntries(Object.entries(surfaces()).map(([w, at]) => [w, at ? Object.getOwnPropertyNames(at).sort() : []]))));`;
  const node = Bun.spawnSync(["node", "--experimental-strip-types", "--input-type=module", "-e", script], { stderr: "pipe" });
  expect(node.exitCode, node.stderr.toString()).toBe(0);
  const nodes: Record<string, string[]> = JSON.parse(node.stdout.toString());
  const ours = names();
  // What this Bun has that Node doesn't, and the reverse: none.
  const extra = Object.entries(ours).flatMap(([w, list]) => list.filter((n) => !nodes[w]?.includes(n)).map((n) => `${w}.${n}`));
  expect(extra).toEqual([]);
});
