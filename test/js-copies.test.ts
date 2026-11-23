// What reaches JS may be changed by JS (#2408): a clone whose copy or
// source a JS caller gets is a copy of its own, as Rust's is, where no Rust
// code changes either, so a clone left out would be one array JS could
// change through both. Each row compiles a crate, and JS changes one value
// and reads the other, which must be as Rust left it. The conservative
// edges are rows of their own: a store through a `&mut`, a push into a
// returned `Vec`, a library's function passing a value on, a callback's
// argument.
import { beforeAll, expect, test } from "bun:test";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`. */
function compiled(dir: string, dep: string, app: string): void {
  mkdirSync(join(dir, "dep"), { recursive: true });
  mkdirSync(join(dir, "app"), { recursive: true });
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
}

const app = `pub struct Match {
    pub players: Vec<String>,
    pub size: usize,
}

pub fn start_match(players: Vec<String>) -> Match {
    let size = players.len();
    Match { players, size }
}

// Both reach JS: the clone is a copy.
pub fn two_matches(players: Vec<String>) -> (Match, Match) {
    let first = start_match(players.clone());
    let second = start_match(players);
    (first, second)
}

// A clone stored through a \`&mut\` into what's returned, and one pushed.
pub fn rosters(players: &Vec<String>) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    push_into(&mut out, players.clone());
    out.push(players.clone());
    out
}

fn push_into(out: &mut Vec<Vec<String>>, roster: Vec<String>) {
    out.push(roster);
}

// A library's function passes its argument on: its result carries it.
pub fn wrapped(players: Vec<String>) -> (Vec<String>, Vec<String>) {
    (dep::pass(players.clone()), players)
}

// A callback JS gave gets a clone: the callback may keep it.
pub fn offer(players: &Vec<String>, take: impl Fn(Vec<String>)) {
    take(players.clone());
}

// Nothing of the clone reaches JS, nor of its source: no copy.
pub fn counted(players: &Vec<String>) -> usize {
    let copy = players.clone();
    copy.len()
}
`;

const dep = `pub fn pass(players: Vec<String>) -> Vec<String> {
    players
}
`;

let out: Record<string, (...args: unknown[]) => unknown>;
let source: string;

beforeAll(async () => {
  const dir = fixture("js-copies");
  compiled(dir, dep, app);
  out = await import(join(dir, "app", "lib.js"));
  source = readFileSync(join(dir, "app", "lib.js"), "utf8");
}, 600_000);

const fn = (name: string) => source.slice(source.indexOf(`export function ${name}(`)).split("\n}")[0]!;

test("two matches JS gets are two: a change to one leaves the other", () => {
  const [first, second] = out.twoMatches!(["ann", "bo"]) as { players: string[] }[];
  first!.players.push("sub");
  expect(second!.players).toEqual(["ann", "bo"]);
});

test("a clone stored through a &mut into what's returned, and one pushed, are copies of JS's array", () => {
  const players = ["ann", "bo"];
  const rosters = out.rosters!(players) as string[][];
  players.push("sub");
  expect(rosters).toEqual([["ann", "bo"], ["ann", "bo"]]);
  rosters[0]!.push("x");
  expect(rosters[1]).toEqual(["ann", "bo"]);
});

test("a library's function passing its argument on carries the clone: a copy", () => {
  const [passed, kept] = out.wrapped!(["ann", "bo"]) as string[][];
  passed!.push("sub");
  expect(kept).toEqual(["ann", "bo"]);
});

test("a callback JS gave gets a copy, which it may keep and change", () => {
  const players = ["ann", "bo"];
  let kept: string[] = [];
  out.offer!(players, (given: string[]) => {
    kept = given;
  });
  kept.push("sub");
  expect(players).toEqual(["ann", "bo"]);
});

test("a clone nothing of reaches JS is left out, as before", () => {
  expect(out.counted!(["ann", "bo"])).toBe(2);
  expect(fn("counted")).not.toContain(".slice()");
});
