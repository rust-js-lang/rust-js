// serde's built crates as compiles read them, a snapshot of what Cargo
// built (#1711): two runs setting serde up at once each find the snapshot
// whole, and a rebuild of what Cargo built, its files written anew, takes
// none from under them.
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { fixture, root } from "./support";
import { keptSerdeFlags } from "./serde-kept";

test("two runs setting serde up at once each compile against a whole snapshot, which a rebuild after takes nothing from", async () => {
  const dir = fixture("serde-kept");
  const built = join(dir, "target", "serde");
  const deps = join(built, "deps");
  mkdirSync(join(deps, "out"), { recursive: true });
  const serde = join(built, "libserde-1.rmeta");
  writeFileSync(serde, "serde's metadata ".repeat(20_000));
  writeFileSync(join(deps, "libserde_core-2.rmeta"), "serde_core's");
  writeFileSync(join(deps, "out", "private.rs"), "a build script's");
  const flags = ["--extern", `serde=${serde}`, "-L", `dependency=${deps}`];
  const store = join(dir, "kept");
  const child = join(dir, "setup.ts");
  writeFileSync(
    child,
    `import { keptSerdeFlags } from ${JSON.stringify(join(root, "test/serde-kept.ts"))};
console.log(JSON.stringify(keptSerdeFlags(${JSON.stringify(flags)}, ${JSON.stringify(store)})));
`,
  );
  // Two at once, as two runs' first serde examples.
  const runs = await Promise.all(
    [0, 1].map(async () => {
      const p = Bun.spawn(["bun", child], { stdout: "pipe", stderr: "pipe" });
      const [out, err, code] = await Promise.all([new Response(p.stdout).text(), new Response(p.stderr).text(), p.exited]);
      expect(code, err).toBe(0);
      return JSON.parse(out) as string[];
    }),
  );
  expect(runs[0]).toEqual(runs[1]);
  expect(runs[0]).toEqual(keptSerdeFlags(flags, store));
  const kept = runs[0];
  expect(kept[0]).toBe("--extern");
  expect(kept[1]).toMatch(new RegExp(`^serde=${store}/[0-9a-f]{24}/0/libserde-1\\.rmeta$`));
  expect(kept[3]).toMatch(new RegExp(`^dependency=${store}/[0-9a-f]{24}/1/deps$`));
  // What Cargo built, rebuilt: its files gone, then written anew.
  rmSync(built, { recursive: true, force: true });
  const keptSerde = kept[1].slice("serde=".length);
  const keptDeps = kept[3].slice("dependency=".length);
  expect(readFileSync(keptSerde, "utf8")).toBe("serde's metadata ".repeat(20_000));
  expect(readFileSync(join(keptDeps, "libserde_core-2.rmeta"), "utf8")).toBe("serde_core's");
  expect(readFileSync(join(keptDeps, "out", "private.rs"), "utf8")).toBe("a build script's");
  // No half-made snapshot left.
  expect(existsSync(store) && require("node:fs").readdirSync(store).some((name: string) => name.includes(".making-"))).toBe(false);
});
