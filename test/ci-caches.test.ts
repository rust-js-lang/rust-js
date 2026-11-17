// The workflows' Actions caches: what each is keyed by, who saves it, and
// how many of each are kept (docs/dev-loop.md).
import { expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./support";

// The workflows' caches are one entry for each set of inputs, not one a
// commit: six caches of some 450 MB an entry, each a push's, held the
// repository's 10 GB full and evicted one another (docs/dev-loop.md).
// And the native programs' kept runs are restored by any run and saved by
// main's alone, which a branch's check can't write over. The test maps are
// one a commit, held to the newest 8 by test-map-prune.yml (ADR 1101,
// amended), about 180 MB.
test("no cache is keyed by its commit but the test maps, held to 8, and only main's runs save the native outcomes", () => {
  const yamls = [".github/workflows", ".github/actions/build-rust-js", ".github/actions/rustc-tests-setup", ".github/actions/native-outcomes"].flatMap((dir) =>
    readdirSync(join(root, dir)).filter((file) => /\.ya?ml$/.test(file)).map((file) => join(dir, file)),
  );
  const keyed = yamls.flatMap((file) =>
    [...readFileSync(join(root, file), "utf8").matchAll(/^\s*key: .*$/gm)].filter((m) => /github\.sha|env\.SHA/.test(m[0])).map((m) => `${file}: ${m[0].trim()}`),
  );
  expect([...new Set(keyed)]).toEqual([".github/workflows/test-map.yml: key: test-map-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-${{ env.SHA }}"]);
  expect(readFileSync(join(root, ".github/workflows/test-map-prune.yml"), "utf8")).toContain('prune-cache --keep=8 --prefix="test-map-${{ runner.os }}-"');
  const outcomes = readFileSync(join(root, ".github/actions/native-outcomes/action.yml"), "utf8");
  expect(outcomes).toContain("- if: inputs.step == 'save' && github.ref == 'refs/heads/main'\n      uses: actions/cache/save@");
  expect(outcomes).not.toMatch(/uses: actions\/cache@/);
  // Each job that restores them saves them.
  for (const file of [".github/workflows/check.yml", ".github/workflows/nightly.yml"]) {
    const text = readFileSync(join(root, file), "utf8");
    expect([file, text.match(/step: restore/g)?.length]).toEqual([file, text.match(/step: save/g)?.length]);
  }
});
