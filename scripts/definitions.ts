// The web platform's definitions bumped by one command, when asked (ADR
// 0024): each package webapi's generator reads, and the TypeScript whose
// DOM lib places `rust_js::declares` (ADR 1152), taken to its newest
// release at least three days old; then webapi regenerated, its coverage
// and tests run, and the React port regenerated, and what changed printed.
// The generator's rules handle what upstream moves (ADR 0024, amended), so
// a bump needs no hand unless coverage loses a member, which this names.
//
//   bun run definitions           check, bump, regenerate, test
//   bun run definitions --check   only list what's newer

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const COOL_OFF_DAYS = 3;
// What a release may be tagged: a stable one, and TypeScript's beta and
// release candidate, which rust-js pins; never `next`, a nightly.
const TAGS = ["latest", "rc", "beta"];
// Each package and the package.json that pins it.
const PINNED: [string, string][] = [
  ["@types/web", "webapi/package.json"],
  ["@webref/idl", "webapi/package.json"],
  ["@webref/elements", "webapi/package.json"],
  ["@webref/events", "webapi/package.json"],
  ["webidl2", "webapi/package.json"],
  ["typescript", "package.json"],
];

function sh(cmd: string[], cwd = root, timeoutMs = 600_000): { ok: boolean; out: string } {
  const run = Bun.spawnSync(cmd, { cwd, timeout: timeoutMs, stdout: "pipe", stderr: "pipe" });
  return { ok: run.exitCode === 0, out: run.stdout.toString() + run.stderr.toString() };
}

function npmView(name: string, field: string): any {
  const { ok, out } = sh(["npm", "view", name, field, "--json"], root, 60_000);
  if (!ok) throw new Error(`npm view ${name} ${field}: ${out}`);
  return JSON.parse(out);
}

/** The newest release of `name` a tag in TAGS names, at least
 * COOL_OFF_DAYS old: of the versions each tag has had, the newest. */
function newest(name: string): string {
  const tags: Record<string, string> = npmView(name, "dist-tags");
  const time: Record<string, string> = npmView(name, "time");
  const old = Date.now() - COOL_OFF_DAYS * 86_400_000;
  // A tag may have moved within the cool-off: the version before it, of the
  // same kind (stable for `latest`, its prerelease kind for the others).
  const kind = (v: string) => v.match(/-([a-z]+)/)?.[1] ?? "stable";
  const kinds = new Set(TAGS.flatMap((t) => (tags[t] ? [kind(tags[t])] : [])));
  const candidates = Object.keys(time)
    .filter((v) => /^\d/.test(v) && kinds.has(kind(v)) && Date.parse(time[v]) <= old)
    .filter((v) => !/-dev\.|-insiders\./.test(v));
  return candidates.sort(Bun.semver.order).at(-1)!;
}

function pinned(file: string, name: string): string {
  const json = JSON.parse(readFileSync(join(root, file), "utf8"));
  return json.devDependencies?.[name] ?? json.dependencies?.[name];
}

function pin(file: string, name: string, from: string, to: string) {
  const path = join(root, file);
  const text = readFileSync(path, "utf8");
  const pinnedAs = `"${name}": "${from}"`;
  if (!text.includes(pinnedAs)) throw new Error(`${file} doesn't pin ${pinnedAs}`);
  writeFileSync(path, text.replace(pinnedAs, `"${name}": "${to}"`));
}

/** What a regeneration of webapi/src/lib.rs added and removed. */
function summary(before: string, after: string): string[] {
  const of = (text: string, re: RegExp) => new Set([...text.matchAll(re)].map((m) => m[1]));
  const lines: string[] = [];
  const diff = (label: string, re: RegExp) => {
    const a = of(before, re);
    const b = of(after, re);
    const added = [...b].filter((x) => !a.has(x)).sort();
    const removed = [...a].filter((x) => !b.has(x)).sort();
    if (added.length) lines.push(`${label} added (${added.length}): ${added.join(", ")}`);
    if (removed.length) lines.push(`${label} removed (${removed.length}): ${removed.join(", ")}`);
  };
  diff("types", /^pub struct (\w+)/gm);
  diff("JS names", /rust_js::link_name = "([^"]+)"/g);
  diff("declared by rust-js", /rust_js::declares\)\]\n(?:#\[[^\n]*\n)*pub struct (\w+)/g);
  return lines;
}

const bumps = PINNED.flatMap(([name, file]) => {
  const from = pinned(file, name);
  const to = newest(name);
  return Bun.semver.order(to, from) > 0 ? [{ name, file, from, to }] : [];
});
if (!bumps.length) {
  console.log(`Every definition is at its newest release ${COOL_OFF_DAYS}+ days old.`);
  process.exit(0);
}
for (const b of bumps) console.log(`${b.name}: ${b.from} -> ${b.to}`);
if (process.argv.includes("--check")) process.exit(0);

for (const b of bumps) pin(b.file, b.name, b.from, b.to);
const failed: string[] = [];
const step = (label: string, cmd: string[], cwd = root, timeoutMs = 600_000) => {
  const { ok, out } = sh(cmd, cwd, timeoutMs);
  console.log(`${ok ? "ok  " : "FAIL"} ${label}`);
  if (!ok) {
    failed.push(label);
    console.log(out.split("\n").slice(-40).join("\n"));
  }
  return out;
};
step("bun install", ["bun", "install"]);
const libRs = join(root, "webapi/src/lib.rs");
const before = readFileSync(libRs, "utf8");
console.log(step("webapi: bun run generate", ["bun", "run", "generate"], join(root, "webapi")).trim());
const after = readFileSync(libRs, "utf8");
for (const line of summary(before, after)) console.log(line);
// Coverage is a ratchet (test/webapi-coverage.test.ts): a member bound
// before and lost now is named, for its rule or its issue; what's gained
// alone is blessed into webapi/coverage.txt.
const { measure, render } = await import("../webapi/coverage");
const baseline = join(root, "webapi/coverage.txt");
const now = await measure();
const covered = new Set(now.covered);
const lost = readFileSync(baseline, "utf8").split("\n").filter((l) => l && !l.startsWith("#") && !covered.has(l));
writeFileSync(baseline, render(now));
if (lost.length) {
  failed.push("coverage");
  console.log(`FAIL coverage: lost ${lost.join(", ")} (blessed in coverage.txt: keep only with an issue tracking each)`);
} else console.log(`ok   coverage: ${now.covered.length} of ${now.all.length} members`);
step("webapi tests", ["bun", "test", "test/webapi-coverage.test.ts", "test/webapi-declares.test.ts", "test/webapi-namespaces.test.ts", "test/webapi-event-map.test.ts"]);
step("React port: regen", ["bun", "scripts/react-in-rust.ts", "regen"], root, 1_800_000);
console.log(failed.length ? `\nNot ready: ${failed.join(", ")}. Fix or report each, then gate as any change.` : "\nReady: gate it as any change (check:fast --all), then commit with the summary above.");
process.exit(failed.length ? 1 : 0);
