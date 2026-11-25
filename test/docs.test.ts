// The guide's examples are what the compiler makes of them
// (docs/guide/README.md#examples): each ```rust fence compiles, and the
// fences after it are its output, as `bun run docs` writes them.
import { expect, test } from "bun:test";
import { readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { underLoad } from "../scripts/timing";
import { buildCompiler, fixtureOf, root } from "./support";

// A test of each page, so a mutation one page reaches runs that page
// alone (`-t <page>`), about 5 s, not the whole guide's 40 s (worker-02 agent's
// `offered-symbol-field-named`). One run of scripts/docs.ts checks every
// page the run's tests are of, at once on the cores, as the whole guide
// was checked; each test says its own page's problems.
const pages = readdirSync(join(root, "docs/guide"), { recursive: true })
  .map(String)
  .filter((page) => page.endsWith(".md"))
  .map((page) => `docs/guide/${page}`)
  .sort();
const named = (page: string) => `the guide's ${page}: its output fences are what the compiler makes`;

/** The pattern `bun test`'s `-t` gives, of `argv`; none of a run of all. */
function namePattern(argv: string[]): RegExp | undefined {
  const at = argv.findIndex((arg) => arg === "-t" || arg === "--test-name-pattern");
  const given = at >= 0 ? argv[at + 1] : argv.find((arg) => arg.startsWith("--test-name-pattern="))?.slice("--test-name-pattern=".length);
  return given === undefined ? undefined : new RegExp(given);
}

/** What a check said, by the page each problem is of, a problem the lines
 * from one that names its page to the next that names one; the rest, of
 * none, under `""`. */
function byPage(said: string, of: string[]): Map<string, string[]> {
  const out = new Map<string, string[]>();
  let current = "";
  for (const line of said.split("\n")) {
    const page = of.find((page) => line.startsWith(`${page}:`));
    if (page !== undefined) {
      current = page;
      out.set(page, [...(out.get(page) ?? []), line]);
    } else if (line.trim() !== "") {
      const at = out.get(current) ?? [];
      if (current === "" || at.length === 0) out.set(current, [...at, line]);
      else at[at.length - 1] += `\n${line}`;
    }
  }
  return out;
}

let checked: { code: number; said: Map<string, string[]> } | undefined;
/** scripts/docs.ts' check of the pages this run's tests are of, once. */
const check = () => {
  if (checked !== undefined) return checked;
  buildCompiler();
  const pattern = namePattern(process.argv);
  const asked = pages.filter((page) => pattern === undefined || pattern.test(named(page)));
  const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check", ...asked.flatMap((page) => ["--page", page])], { cwd: root, timeout: 280_000 });
  checked = { code: result.exitCode ?? 1, said: byPage(result.stderr.toString(), pages) };
  return checked;
};

for (const page of pages) {
  test(named(page), () => {
    const { code, said } = check();
    // Its page's problems; where the run failed and said none of a page's,
    // all it said.
    const anyPage = [...said.keys()].some((key) => key !== "");
    expect(code === 0 || anyPage ? (said.get(page) ?? []) : [...said.values()].flat()).toEqual([]);
  }, 290_000);
}

test("a guide page's test is asked by its name alone", () => {
  expect(namePattern(["/usr/local/bin/bun", "test", "test/docs.test.ts", "-t", "vite.md"])?.test(named("docs/guide/tutorial/vite.md"))).toBe(true);
  expect(namePattern(["/usr/local/bin/bun", "test", "--test-name-pattern=vite.md"])?.source).toBe("vite.md");
  expect(namePattern(["/usr/local/bin/bun", "test", "test/docs.test.ts"])).toBeUndefined();
  expect(pages).toContain("docs/guide/tutorial/vite.md");
  // A problem's lines are its page's, to the next that names a page.
  const said = byPage("docs/guide/a.md: the example at fence 1 doesn't compile:\nerror: no\n  --> x.rs\ndocs/guide/b.md: 1 output fence\nnot what the compiler makes\n", ["docs/guide/a.md", "docs/guide/b.md"]);
  expect([...said]).toEqual([
    ["docs/guide/a.md", ["docs/guide/a.md: the example at fence 1 doesn't compile:\nerror: no\n  --> x.rs"]],
    ["docs/guide/b.md", ["docs/guide/b.md: 1 output fence\nnot what the compiler makes"]],
  ]);
});

// Each run compiles an example in a directory of its own, claimed as a
// test's fixture is, so two runs in one checkout don't remove each other's
// as they compile (#2198): with another run's claim held on the first,
// `_0`, this run compiles in another, the one it says it kept, and leaves
// `_0` as it was. Which, and whether it's there after: neither is this
// test's, as another guide run may claim `_1` first, and another
// process's sweep removes a kept one once its process is gone (#2422).
test("a guide run compiles in its own directory, leaving another run's", () => {
  buildCompiler();
  // Another run's claim, taken as a run takes its own, on the first index
  // no live process holds: a guide run beside it holds one too.
  const held = fixtureOf("docs.ts", "docs-tutorial_vite.md-0");
  writeFileSync(join(held, "another-run.txt"), "held");
  let mine: string | undefined;
  try {
    const result = Bun.spawnSync([process.execPath, "scripts/docs.ts", "--check", "--page", "docs/guide/tutorial/vite.md"], {
      cwd: root,
      env: { ...process.env, RUST_JS_KEEP_FIXTURES: "1" },
      timeout: underLoad(120_000),
    });
    expect(result.exitCode, result.stderr.toString()).toBe(0);
    expect(readFileSync(join(held, "another-run.txt"), "utf8")).toBe("held");
    mine = /^kept (.*docs-tutorial_vite\.md-0_\d+)$/m.exec(result.stderr.toString())?.[1];
    expect(mine).toBeDefined();
    expect(mine).not.toBe(held);
    expect(mine!.slice(0, mine!.lastIndexOf("_"))).toBe(held.slice(0, held.lastIndexOf("_")));
  } finally {
    if (mine !== undefined) {
      rmSync(mine, { recursive: true, force: true });
      rmSync(`${mine}.owner`, { force: true });
    }
  }
}, underLoad(130_000));
