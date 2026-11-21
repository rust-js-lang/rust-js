// React's port's checkout, a cache each regen checks out at its pin
// (scripts/react-in-rust.ts): one regen's at a time, held by its lock,
// which another checkout's bounding leaves alone, and made again from
// nothing once where its objects are unreadable (#2279): a retirement's
// regen beside a check's left one so, and every push failed at its React
// gate, "unable to read sha1 file".

import { expect, test } from "bun:test";
import { mkdirSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { acquire } from "../scripts/leases";
import { bound, checkoutPin, pinCheckedOut } from "../scripts/react-in-rust";
import { underLoad } from "../scripts/timing";
import { fixture, root } from "./support";

// Never past the fixture: a broken repository there is never the
// checkout's own, which a first draft of this test reached and changed.
const git = (cwd: string, ...args: string[]) => {
  const done = runSync(["git", ...args], cwd, 60_000, { GIT_CEILING_DIRECTORIES: join(root, "target"), GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@example.com", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@example.com" });
  expect(done.code, done.stderr).toBe(0);
  return done.stdout.trim();
};

test("a port's checkout whose objects are unreadable is made again once, saying so", () => {
  const dir = fixture("react-port-checkout");
  rmSync(dir, { recursive: true, force: true });
  const origin = join(dir, "origin");
  mkdirSync(origin, { recursive: true });
  git(origin, "init", "-q");
  git(origin, "config", "uploadpack.allowAnySHA1InWant", "true");
  writeFileSync(join(origin, "a.js"), "export const a = 1;\n");
  git(origin, "add", "a.js");
  git(origin, "commit", "-q", "-m", "a");
  const pin = git(origin, "rev-parse", "HEAD");
  const port = join(dir, "home", "react-in-rust");
  const checkout = () => [git(root, "remote", "get-url", "origin"), git(root, "rev-parse", "HEAD"), git(root, "rev-parse", "--is-shallow-repository")];
  const before = checkout();
  expect(checkoutPin(port, origin, pin)).toBeUndefined();
  expect(readFileSync(join(port, "a.js"), "utf8")).toBe("export const a = 1;\n");
  // Its repository unreadable, as a stopped or concurrent run leaves one.
  writeFileSync(join(port, ".git", "HEAD"), "garbage\n");
  expect(checkoutPin(port, origin, pin)).toBeString();
  const said: string[] = [];
  pinCheckedOut(port, origin, pin, (line) => said.push(line));
  expect(said).toHaveLength(1);
  expect(said[0]).toStartWith("react-in-rust: the port's checkout failed (");
  expect(said[0]).toEndWith("): made again from nothing");
  expect(git(port, "rev-parse", "HEAD")).toBe(pin);
  expect(readFileSync(join(port, "a.js"), "utf8")).toBe("export const a = 1;\n");
  // One as it should be is checked out as it is, nothing said.
  pinCheckedOut(port, origin, pin, (line) => said.push(line));
  expect(said).toHaveLength(1);
  // And the checkout's own repository as it was.
  expect(checkout()).toEqual(before);
}, underLoad(120_000));

test("another checkout's port a regen holds isn't bounded from under it", () => {
  const dir = fixture("react-port-held");
  rmSync(dir, { recursive: true, force: true });
  const own = join(dir, "react-in-rust-00000000");
  const held = join(dir, "react-in-rust-11111111");
  const idle = join(dir, "react-in-rust-22222222");
  const old = new Date(Date.now() - 3 * 3600 * 1000);
  for (const at of [own, held, idle]) {
    mkdirSync(at, { recursive: true });
    writeFileSync(join(at, "used"), "");
    utimesSync(join(at, "used"), old, old);
    utimesSync(at, old, old);
  }
  const release = acquire(join(held, "port.lock"))!;
  try {
    // Every idle one past a cap of nothing goes; the held one stays.
    const removed = bound({ dir, own, cap: 0, free: () => 0 });
    expect(removed).toContain(idle);
    expect(removed).not.toContain(held);
  } finally {
    release();
  }
  expect(readFileSync(join(root, "scripts/react-in-rust.ts"), "utf8")).toContain("const released = acquire(portLock(home)");
});

// Every git step of the port names its own repository (`portRepository`),
// a `step` of `checkoutPin` by its own, so a new one can't drop it unseen:
// one that didn't, in a port whose `.git` was unreadable, reached the
// checkout's own and set its origin and HEAD (#2279). Only rust-js's own
// log, run in its root, is of another repository.
test("each git step of React's port names the port's own repository", () => {
  const source = readFileSync(join(root, "scripts/react-in-rust.ts"), "utf8");
  expect(source).toContain(`const done = run(cmd, cwd, cmd[1] === "init" ? {} : own);`);
  const unnamed = source
    .split("\n")
    .map((line, i) => [line, i + 1] as const)
    .filter(([line]) => line.includes(`["git", `))
    .filter(([line]) => !/\bstep\(\["git", /.test(line) || /\b(run|must)\(\["git", /.test(line))
    .filter(([line]) => !/, (own|portRepository\((port|at)\))\)/.test(line) && !/\], root\)/.test(line))
    .map(([line, at]) => `${at}: ${line.trim()}`);
  expect(unnamed).toEqual([]);
});
