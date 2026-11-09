// rust-js's own wasm-bindgen, js-sys and web-sys keep up with crates.io's
// (ADR 1112): each at the newest release of its minor, so a dependency's
// `^` requirement, `wasm-bindgen = "0.2.107"`, takes it. One behind, Cargo
// takes crates.io's, which rust-js can't compile. Nightly's: it reads
// crates.io's index.

import { expect, test } from "bun:test";
import { compatCrates } from "../tooling/patch.js";
import { underLoad } from "../scripts/timing";

/** The newest release of crates.io's `name` whose minor is `version`'s. */
async function newest(name: string, version: string): Promise<string> {
  const path = name.length <= 3 ? `${name.length}/${name}` : `${name.slice(0, 2)}/${name.slice(2, 4)}/${name}`;
  const response = await fetch(`https://index.crates.io/${path}`);
  expect(response.status).toBe(200);
  const minor = version.split(".").slice(0, 2).join(".");
  const releases = (await response.text())
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as { vers: string; yanked: boolean })
    .filter((r) => !r.yanked && !r.vers.includes("-") && r.vers.startsWith(`${minor}.`))
    .map((r) => r.vers);
  return releases.sort((a, b) => Number(a.split(".")[2]) - Number(b.split(".")[2])).at(-1)!;
}

test("rust-js's own wasm-bindgen family is at crates.io's newest release of its minor", async () => {
  const behind: string[] = [];
  for (const [name, { version }] of compatCrates()) {
    const latest = await newest(name, version);
    if (latest !== version) behind.push(`tooling/compat/${name}/Cargo.toml: version = "${latest}", crates.io's newest, where it's "${version}"`);
  }
  expect(behind).toEqual([]);
}, underLoad(60_000));
