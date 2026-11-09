// Miri, which runs a program and finds what it does that's undefined
// behaviour: the one judge of whether a program is one to compare with its
// JS at all. The generated programs ask it with `FUZZ_MIRI=1` (#1085), and
// the reducer of each step that keeps `unsafe` code (#1104): a program that
// is undefined behaviour prints what it happens to, natively, and its JS
// differing from that is no bug. The nightly toolchain's Miri, of the
// sysroot `cargo miri setup` makes once:
//
//   rustup toolchain install nightly --profile minimal --component miri,rust-src
//   cargo +nightly miri setup
import { spawnSync } from "node:child_process";

import { root } from "./support";

/** How to install Miri, where it's asked for and isn't. */
export const installMiri = "rustup toolchain install nightly --profile minimal --component miri,rust-src && cargo +nightly miri setup";

/** Whether Miri is installed here: asked once. */
let installed: boolean | undefined;
export function miriInstalled(): boolean {
  installed ??= spawnSync("rustup", ["run", "nightly", "miri", "--version"], { encoding: "utf8", timeout: 60_000 }).status === 0;
  return installed;
}

/** Miri's sysroot, `cargo miri setup`'s: `MIRI_SYSROOT` where it's given,
 * which Miri reads itself, so a run asks for it once and the processes it
 * starts are given it. */
export function miriSysroot(): string {
  const given = process.env.MIRI_SYSROOT;
  if (given) return given;
  const p = spawnSync("cargo", ["+nightly", "miri", "setup", "--print-sysroot"], { cwd: root, encoding: "utf8", timeout: 900_000 });
  const said = (p.stdout ?? "").trim().split("\n").pop() ?? "";
  if (p.status !== 0 || said === "") throw new Error(`Miri's sysroot isn't set up: ${p.stderr}; install it: ${installMiri}`);
  process.env.MIRI_SYSROOT = said;
  return said;
}

/** What Miri finds of `file`'s program that's undefined behaviour, or
 * nothing: its run's own ends, a panic or an exit, are native Rust's to
 * compare. One that runs past its time says nothing of it either. */
export function undefinedBehaviour(file: string, edition = "2024"): string | undefined {
  const p = spawnSync("rustup", ["run", "nightly", "miri", "--sysroot", miriSysroot(), `--edition=${edition}`, "-Awarnings", file], {
    cwd: root,
    encoding: "utf8",
    timeout: 300_000,
    maxBuffer: 64 * 1024 * 1024,
  });
  const said = `${p.stderr ?? ""}`;
  return said.includes("Undefined Behavior") ? said : undefined;
}

/** Whether `text`, a program's, may be undefined behaviour, which only
 * `unsafe` code can be: a raw pointer read, `alloc`, `MaybeUninit`'s
 * `assume_init`, `transmute`, each is `unsafe` to write. Safe Rust's
 * programs are Rust's to keep defined. */
export function mayBeUndefined(text: string): boolean {
  return /\bunsafe\b/.test(text);
}
