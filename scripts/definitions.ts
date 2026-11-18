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
// What a release may be tagged: a stable one, and a beta or a release
// candidate only where a capability below asks it; never `next`, a nightly.
const TAGS = ["latest", "rc", "beta"];
// Of each package, the capabilities no stable release supplies yet, each with
// the test that fails without it: only these make its prerelease eligible.
// The newest stable release is taken otherwise (#1903).
export const PRERELEASE_FOR: Record<string, { capability: string; test: string }[]> = {
  typescript: [
    {
      capability: "the DOM library declares what webapi's generator reads, CloseWatcher, DocumentPictureInPicture, Serial, SerialPort and the rest, and their event maps, which 7.0's doesn't",
      test: "test/definitions.test.ts -t \"the pinned TypeScript's DOM library\"",
    },
  ],
};
// Each package and the package.json that pins it.
const PINNED: [string, string][] = [
  ["@types/web", "webapi/package.json"],
  ["@webref/idl", "webapi/package.json"],
  ["@webref/elements", "webapi/package.json"],
  ["@webref/events", "webapi/package.json"],
  ["webidl2", "webapi/package.json"],
  ["typescript", "package.json"],
];

async function sh(cmd: string[], cwd = root, timeoutMs = 600_000): Promise<{ ok: boolean; out: string }> {
  const run = Bun.spawn(cmd, { cwd, timeout: timeoutMs, stdout: "pipe", stderr: "pipe" });
  const [code, stdout, stderr] = await Promise.all([run.exited, new Response(run.stdout).text(), new Response(run.stderr).text()]);
  return { ok: code === 0, out: stdout + stderr };
}

async function packageMetadata(name: string, field: string): Promise<any> {
  const { ok, out } = await sh(["bun", "pm", "view", name, field, "--json"], root, 60_000);
  if (!ok) throw new Error(`bun pm view ${name} ${field}: ${out}`);
  return JSON.parse(out);
}

/** The newest stable release a tag in TAGS names, at least COOL_OFF_DAYS
 * old: of the versions each tag has had, the newest. A beta or a release
 * candidate is eligible only where `prerelease` says a capability needs it,
 * `PRERELEASE_FOR`'s, and then the newest of either. */
export function newest(tags: Record<string, string>, time: Record<string, string>, now = Date.now(), prerelease = false): string {
  const old = now - COOL_OFF_DAYS * 86_400_000;
  // A tag may have moved within the cool-off: the version before it, of the
  // same kind (stable for `latest`, its prerelease kind for the others).
  const kind = (v: string) => v.match(/-([a-z]+)/)?.[1] ?? "stable";
  const kinds = new Set(TAGS.flatMap((t) => (tags[t] ? [kind(tags[t])] : [])).filter((k) => prerelease || k === "stable"));
  const candidates = Object.keys(time)
    .filter((v) => /^\d/.test(v) && kinds.has(kind(v)) && Date.parse(time[v]) <= old)
    .filter((v) => !/-dev\.|-insiders\./.test(v));
  const version = candidates.sort(Bun.semver.order).at(-1);
  if (!version) throw new Error("No eligible release is old enough under the accepted tags");
  return version;
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

async function main() {
  const bumps = (await Promise.all(PINNED.map(async ([name, file]) => {
    const from = pinned(file, name);
    const [tags, time] = await Promise.all([packageMetadata(name, "dist-tags"), packageMetadata(name, "time")]);
    const to = newest(tags, time, Date.now(), (PRERELEASE_FOR[name] ?? []).length > 0);
    return Bun.semver.order(to, from) > 0 ? [{ name, file, from, to }] : [];
  }))).flat();
  // `--regenerate`: every step at the pins as they are, which proves the
  // regeneration deterministic, nothing in the tree moved (#1903).
  if (!bumps.length && !process.argv.includes("--regenerate")) {
    console.log(`Every definition is at its newest release ${COOL_OFF_DAYS}+ days old.`);
    process.exit(0);
  }
  for (const b of bumps) console.log(`${b.name}: ${b.from} -> ${b.to}`);
  // A stable release where a prerelease was asked: its capability's test
  // may pass on it now, and the entry go.
  for (const b of bumps) {
    for (const asked of PRERELEASE_FOR[b.name] ?? []) {
      if (!b.to.includes("-")) console.log(`${b.name} ${b.to} is stable: if \`bun test ${asked.test}\` passes on it, remove its PRERELEASE_FOR entry (${asked.capability})`);
    }
  }
  if (process.argv.includes("--check")) process.exit(0);

  for (const b of bumps) {
    // The published parser dependency consumes the root engine pin, never another release.
    if (b.name === "typescript") {
      const parser = pinned("typescript/package.json", b.name);
      if (parser !== b.from) throw new Error("The parser and root TypeScript pins differ");
      pin("typescript/package.json", b.name, parser, b.to);
    }
    pin(b.file, b.name, b.from, b.to);
  }
  const failed: string[] = [];
  const step = async (label: string, cmd: string[], cwd = root, timeoutMs = 600_000) => {
    const { ok, out } = await sh(cmd, cwd, timeoutMs);
    console.log(`${ok ? "ok  " : "FAIL"} ${label}`);
    if (!ok) {
      failed.push(label);
      console.log(out.split("\n").slice(-40).join("\n"));
    }
    return out;
  };
  await step("bun install", ["bun", "install"]);
  const libRs = join(root, "webapi/src/lib.rs");
  const before = readFileSync(libRs, "utf8");
  console.log((await step("webapi: bun run generate", ["bun", "run", "generate"], join(root, "webapi"))).trim());
  const after = readFileSync(libRs, "utf8");
  for (const line of summary(before, after)) console.log(line);
  // These existing generators remain their groups’ authorities until #1904 migrates them.
  await Promise.all(["typed_arrays", "errors", "intl"].map(name => step("builtins: " + name, ["bun", "builtins/" + name + ".ts"])));
  const coverage = async <C extends { all: string[]; covered: string[] }>(name: string, model: { measure: () => Promise<C>; render: (value: C) => string }) => {
    const baseline = join(root, name, "coverage.txt");
    const now = await model.measure();
    const covered = new Set(now.covered);
    const before = readFileSync(baseline, "utf8").split("\n").filter(line => line && !line.startsWith("#"));
    const lost = before.filter(member => !covered.has(member));
    const added = now.covered.filter(member => !before.includes(member));
    if (lost.length) {
      failed.push(name + " coverage");
      console.log(`FAIL ${name} coverage: lost ${lost.join(", ")}; baseline retained`);
    } else {
      writeFileSync(baseline, model.render(now));
      console.log(`ok   ${name} coverage: ${now.covered.length} of ${now.all.length} members`);
    }
    if (added.length) console.log(name + " covered additions: " + added.join(", "));
    const missing = now.all.filter(member => !covered.has(member));
    if (missing.length) console.log(name + " unbound: " + missing.join(", "));
  };
  await Promise.all([
    import("../webapi/coverage").then(model => coverage("webapi", model)),
    import("../builtins/coverage").then(model => coverage("builtins", model)),
  ]);
  // Each gap's issue still open: a closed one names a gap nothing tracks
  // (builtins/gaps.txt, #1903).
  const { gaps } = await import("../builtins/gaps");
  for (const issue of [...new Set(gaps().map((g) => g.issue))].sort((a, b) => a - b)) {
    const { ok, out } = await sh(["gh", "issue", "view", String(issue), "--json", "state", "--jq", ".state"], root, 60_000);
    if (!ok || out.trim() !== "OPEN") {
      failed.push(`gap issue #${issue}`);
      console.log(`FAIL builtins/gaps.txt: #${issue} is ${ok ? out.trim().toLowerCase() : `unread: ${out.trim()}`}; bind its gaps, or name the issue that tracks them`);
    }
  }
  await step("builtin tests", ["bun", "test", "--parallel", "test/builtins-coverage.test.ts", "test/intl-arguments.test.ts"]);
  await step("webapi tests", ["bun", "test", "test/webapi-coverage.test.ts", "test/webapi-declares.test.ts", "test/webapi-namespaces.test.ts", "test/webapi-event-map.test.ts"]);
  await step("React port: regen", ["bun", "scripts/react-in-rust.ts", "regen"], root, 1_800_000);
  console.log(failed.length ? `\nNot ready: ${failed.join(", ")}. Fix or report each, then gate as any change.` : "\nReady: gate it as any change (check:fast --all), then commit with the summary above.");
  process.exit(failed.length ? 1 : 0);
}

if (import.meta.main) await main();
