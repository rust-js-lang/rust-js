// createDetails' whole shape, of ADR 1164's forms, held to Codex's contract
// (rust-js-lang/vercel-ui-sdk-in-rust@82369d8, docs/create-details-contract.md,
// tests/change-details.test.ts and tests/types/change-details.ts) against an
// excerpt of upstream's kernel/details.ts (vercel-labs/ui@220c1a0): the type
// map's vocabulary, the distributive details, the `const` given, the
// boolean `cancelable` and its default, the spread, the live `canceled`,
// and an unchecked cast whose declared type is upstream's own. Its JS takes
// upstream's three arguments, no dictionary of `C` (#1915, ADR 1165). Codex
// writes the port's function; this holds the compiler to it.

import { beforeAll, expect, test } from "bun:test";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { contract, type Factory } from "./create-details/contract";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

const sources = join(import.meta.dir, "create-details");
const withWeb = ["--", "--crate-type=lib", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** The compiled module and its declarations, beside upstream's excerpt. */
function compiled(): string {
  const dir = fixture("create-details");
  copyFileSync(join(sources, "details.rs"), join(dir, "lib.rs"));
  copyFileSync(join(sources, "reference.ts"), join(dir, "reference.ts"));
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  return dir;
}

/** TypeScript's errors of `file` in `dir`, as Codex's declarations test
 * checks it: strict, the DOM's lib in. */
function typeChecked(dir: string, file: string): string[] {
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "--ignoreConfig", "--noEmit", "--strict", "--skipLibCheck", "--target", "esnext", "--module", "esnext", "--moduleResolution", "bundler", "--lib", "esnext,dom", "--allowImportingTsExtensions", file], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

test("createDetails' JS is upstream's body, of its three arguments", () => {
  const dir = compiled();
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  // The getter and the spread are upstream's; `cancel` is a method where
  // upstream writes an arrow property, which only `String(cancel)` tells
  // (#1931); and `!(cancelable == null || cancelable)` is upstream's
  // `!cancelable` of its default `cancelable = true` (#1910).
  expect(js).toContain(`export function createDetails(given, source, cancelable) {
  if (!(cancelable == null || cancelable)) {
    return {
      ...given,
      source,
    };
  }
  let canceled = false;
  return {
    ...given,
    source,
    cancel() {
      canceled = true;
    },
    get canceled() {
      return canceled;
    },
  };
}`);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain(`}, C extends boolean = true>(given: G, source: ChangeSource, cancelable?: C | undefined): (C extends true ? ChangeDetails<G["reason"]> : CommitDetails<G["reason"]>) & Omit<G, "reason" | "event" | "source">;`);
  expect(declarations).toContain("export function createDetails<const G extends {\n    reason: string;\n    event?: Event;\n}");
  expect(declarations).not.toContain("type Details");
});

test("createDetails runs Codex's contract as upstream's does", async () => {
  const dir = compiled();
  const ours = (await import(join(dir, "lib.js"))).createDetails as Factory;
  const theirs = (await import(join(dir, "reference.ts"))).createDetails as Factory;
  const rows = contract(ours);
  expect(rows.length).toBeGreaterThan(20);
  expect(rows).toEqual(contract(theirs));
  // Upstream's default is of an omitted or `undefined` argument alone: its
  // `null`, which both declarations refuse, makes a commit, where ours,
  // whose body tests `== null`, makes a change (#1910).
  const cancels = (create: Factory) => Object.hasOwn(create({ reason: "press" }, "mouse", null as never), "cancel");
  expect([cancels(ours), cancels(theirs)]).toEqual([true, false]);
  // `cancel` is a method of ours, upstream's an arrow (#1931): neither has a
  // prototype, nor reads `this`.
  const [method, arrow] = [ours, theirs].map((create) => create({ reason: "press" }, "mouse").cancel as () => void);
  expect([String(method).startsWith("cancel()"), String(arrow).startsWith("() =>"), "prototype" in method!, "prototype" in arrow!]).toEqual([true, true, false, false]);
});

test("createDetails' declarations are Codex's type contract", () => {
  const dir = compiled();
  writeFileSync(join(dir, "consumer.ts"), `import * as Candidate from "./lib.js";
import * as Reference from "./reference.ts";

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2) ? true : false;
type Assert<T extends true> = T;
type Sources = Assert<Equal<Candidate.ChangeSource, Reference.ChangeSource>>;
type DefaultChange = Assert<Equal<Candidate.ChangeDetails, Reference.ChangeDetails>>;
type DefaultCommit = Assert<Equal<Candidate.CommitDetails, Reference.CommitDetails>>;
const constructor: typeof Reference.createDetails = Candidate.createDetails;
const referenceConstructor: typeof Candidate.createDetails = Reference.createDetails;
type Vocabulary = Assert<Equal<Candidate.ReasonVocabulary, Reference.ReasonVocabulary>>;
type Reasons = Assert<Equal<Candidate.ChangeReason, Reference.ChangeReason>>;
type Changes = Assert<Equal<Candidate.ChangeDetails<Reference.ChangeReason>, Reference.ChangeDetails<Reference.ChangeReason>>>;
type Commits = Assert<Equal<Candidate.CommitDetails<Reference.ChangeReason>, Reference.CommitDetails<Reference.ChangeReason>>>;

const change = Candidate.createDetails({reason:"escape-key", row:7}, "keyboard");
const row: 7 = change.row;
const event: KeyboardEvent | undefined = change.event;
const stopped: boolean = change.canceled;
change.cancel();
// @ts-expect-error the const generic preserves readonly extra fields
change.row = 9;
// @ts-expect-error canceled is a live readonly property
change.canceled = true;

const commit = Candidate.createDetails({reason:"input-commit", row:7}, "programmatic", false);
const commitEvent: FocusEvent | KeyboardEvent | undefined = commit.event;
// @ts-expect-error a commit created without cancellation has no cancel method
commit.cancel();
// @ts-expect-error a commit created without cancellation has no canceled property
commit.canceled;
const custom = Candidate.createDetails({reason:"custom-reason"}, "agent");
const customEvent: Event | undefined = custom.event;
// The original constructor trusts the reason/event pair; do not silently narrow its input.
Candidate.createDetails({reason:"escape-key", event:new MouseEvent("click")}, "mouse");
// @ts-expect-error invalid source
Candidate.createDetails({reason:"press"}, "invalid");
// @ts-expect-error a reason is required
Candidate.createDetails({}, "mouse");

declare const details: Candidate.ChangeDetails<Reference.ChangeReason>;
if (details.reason === "escape-key") {
  const key: string | undefined = details.event?.key;
}
if (details.reason === "drop") {
  const transfer: DataTransfer | null | undefined = details.event?.dataTransfer;
}
`);
  expect(typeChecked(dir, "consumer.ts")).toEqual([]);
});
