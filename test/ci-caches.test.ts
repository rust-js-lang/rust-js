// The workflows' Actions caches: what each is keyed by, who saves it, and
// how many of each are kept (docs/dev-loop.md).
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { cacheFamily, cachePast } from "../scripts/ci";
import { root } from "./support";

// The workflows' caches are one entry for each set of inputs, not one a
// commit: six caches of some 450 MB an entry, each a push's, held the
// repository's 10 GB full and evicted one another (docs/dev-loop.md).
// And the native programs' kept runs are restored by any run and saved by
// main's alone, which a branch's check can't write over. The test maps are
// one a commit; they, the binaries, one a compiler change, and the native
// stores, one a run, are held to each family's newest 8 by cache-prune.yml,
// after each run that saves them: 420 entries, 4.6 GB, had built up.
test("no cache is keyed by its commit but the test maps, each family held to 8, and only main's runs save the native outcomes", () => {
  const yamls = [".github/workflows", ".github/actions/build-rust-js", ".github/actions/rustc-tests-setup", ".github/actions/native-outcomes"].flatMap((dir) =>
    readdirSync(join(root, dir)).filter((file) => /\.ya?ml$/.test(file)).map((file) => join(dir, file)),
  );
  const keyed = yamls.flatMap((file) =>
    [...readFileSync(join(root, file), "utf8").matchAll(/^\s*key: .*$/gm)].filter((m) => /github\.sha|env\.SHA/.test(m[0])).map((m) => `${file}: ${m[0].trim()}`),
  );
  expect([...new Set(keyed)]).toEqual([".github/workflows/test-map.yml: key: test-map-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ env.SHA }}"]);
  const prune = Bun.YAML.parse(readFileSync(join(root, ".github/workflows/cache-prune.yml"), "utf8")) as { on: { workflow_run: { workflows: string[] } }; jobs: { prune: { "runs-on": string; steps: { run?: string }[] } } };
  expect(prune.jobs.prune.steps.map((step) => step.run).filter(Boolean)).toEqual(["bun scripts/ci.ts prune-cache"]);
  expect(prune.jobs.prune["runs-on"]).toBe("ubuntu-latest");
  // It runs after each of the three that save entries a run or a commit,
  // by their names; it prunes every family, the others' too.
  const named = (file: string) => (Bun.YAML.parse(readFileSync(join(root, ".github/workflows", file), "utf8")) as { name: string }).name;
  expect(prune.on.workflow_run.workflows.toSorted()).toEqual(["check.yml", "nightly.yml", "test-map.yml"].map(named).toSorted());
  const outcomes = readFileSync(join(root, ".github/actions/native-outcomes/action.yml"), "utf8");
  expect(outcomes).toContain("- if: inputs.step == 'save' && github.ref == 'refs/heads/main'\n      uses: actions/cache/save@");
  expect(outcomes).not.toMatch(/uses: actions\/cache@/);
  // Each job that restores them saves them.
  for (const file of [".github/workflows/check.yml", ".github/workflows/nightly.yml"]) {
    const text = readFileSync(join(root, file), "utf8");
    expect([file, text.match(/step: restore/g)?.length]).toEqual([file, text.match(/step: save/g)?.length]);
  }
});

// A cache family is its key less what each run or set of inputs makes new:
// trailing hashes, `hashFiles`' and a commit's, and a run's id and attempt,
// never a name's own number, a shard's. Each ref's family keeps its newest.
test("the cache prune keeps each family's newest 8, by ref", () => {
  const hash = (c: string) => c.repeat(64);
  expect(cacheFamily(`rust-js-release-Linux-${hash("a")}`)).toBe("rust-js-release-Linux");
  expect(cacheFamily("native-outcomes-Linux-test-1-of-2-44113256330-1")).toBe("native-outcomes-Linux-test-1-of-2");
  expect(cacheFamily("native-outcomes-Linux-stages-3-44113256330-2")).toBe("native-outcomes-Linux-stages-3");
  expect(cacheFamily("native-outcomes-Linux-test-1-of-2-nightly-44001832185-1")).toBe("native-outcomes-Linux-test-1-of-2-nightly");
  expect(cacheFamily(`test-map-Linux-${hash("d")}-${"7f324c2e".repeat(5)}`)).toBe("test-map-Linux");
  expect(cacheFamily(`test-map-Linux-${hash("d")}-44113256330`)).toBe("test-map-Linux");
  expect(cacheFamily(`wasm-check-Linux-${hash("b")}-${hash("c")}`)).toBe("wasm-check-Linux");
  expect(cacheFamily("bun-vyrKTwhw5PEh6pGRz1Zgyle11uQ=")).toBe("bun-vyrKTwhw5PEh6pGRz1Zgyle11uQ=");
  const entry = (id: number, key: string, ref = "refs/heads/main") => ({ id, key, ref, createdAt: new Date(Date.UTC(2026, 10, 17, 0, id)).toISOString(), sizeInBytes: 1 });
  const binaries = Array.from({ length: 11 }, (_, i) => entry(i, `rust-js-debug-Linux-${hash(i.toString(16))}`));
  const stages = Array.from({ length: 9 }, (_, i) => entry(20 + i, `native-outcomes-Linux-stages-0-${44000000000 + i}-1`));
  const branch = Array.from({ length: 3 }, (_, i) => entry(40 + i, `rust-js-debug-Linux-${hash("f")}`, `refs/heads/b${i}`));
  const past = cachePast([...binaries, ...stages, ...branch, entry(50, "native-outcomes-Linux-stages-1-44000000000-1")], 8);
  expect(past.map(({ id }) => id).toSorted((a, b) => a - b)).toEqual([0, 1, 2, 20]);
});
