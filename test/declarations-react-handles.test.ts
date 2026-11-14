// What a React hook gives back that a crate exports, a state's setter, a
// reducer's dispatch, a ref, a forwarded ref and an optimistic setter, is
// declared as @types/react names it (#1763), where each was `any`: a
// TypeScript caller is checked against it, and the JS gives on the very
// function it was given.

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, root, run, target } from "./support";

beforeAll(buildReact, 600_000);

/** TypeScript's errors in `file` of `dir`, strict, its declarations checked too. */
function typeChecked(dir: string, file: string): string[] {
  const config = join(dir, "tsconfig.json");
  writeFileSync(config, JSON.stringify({
    compilerOptions: { module: "esnext", moduleResolution: "bundler", target: "es2022", strict: true, noEmit: true, skipLibCheck: false, exactOptionalPropertyTypes: true, typeRoots: [join(root, "node_modules/@types")] },
    files: [file],
  }));
  const checked = Bun.spawnSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], { cwd: dir });
  return checked.stdout.toString().split("\n").filter((line) => line.includes("error TS"));
}

test("a hook's setter, dispatch and refs are declared as @types/react names them", async () => {
  const dir = fixture("declarations-react-handles");
  writeFileSync(join(dir, "lib.rs"), `use react::{Dispatch, ForwardedRef, RefObject, SetOptimistic, SetStateAction};

pub struct State {
    pub set_value: &'static Dispatch<SetStateAction<u32>>,
}

pub fn keep(state: State) -> State {
    state
}

pub fn dispatcher(dispatch: Dispatch<String>) -> Dispatch<String> {
    dispatch
}

pub fn holder(held: RefObject<Option<u32>>) -> RefObject<Option<u32>> {
    held
}

pub fn forwarded(given: ForwardedRef<u32>) -> ForwardedRef<u32> {
    given
}

pub fn optimistic(set: SetOptimistic<u32>) -> SetOptimistic<u32> {
    set
}
`);
  run([compiler, "--declarations", join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target]);
  const declarations = readFileSync(join(dir, "lib.d.ts"), "utf8");
  expect(declarations).toContain("setValue: Dispatch<SetStateAction<number>>;");
  expect(declarations).toContain("export function dispatcher(dispatch: Dispatch<string>): Dispatch<string>;");
  expect(declarations).toContain("export function holder(held: RefObject<number | null | undefined>): RefObject<number | null | undefined>;");
  expect(declarations).toContain("export function forwarded(given: ForwardedRef<number>): ForwardedRef<number>;");
  expect(declarations).toContain("export function optimistic(set: (action: number | ((pendingState: number) => number)) => void): (action: number | ((pendingState: number) => number)) => void;");
  expect(declarations).toContain('import type { Dispatch, ForwardedRef, RefObject, SetStateAction } from "react";');
  expect(declarations).not.toContain("any");
  writeFileSync(join(dir, "use.ts"), `import type { Dispatch, SetStateAction } from "react";
import { dispatcher, holder, keep, optimistic } from "./lib.js";
declare const setCount: Dispatch<SetStateAction<number>>;
const kept = keep({ setValue: setCount });
kept.setValue(1);
kept.setValue((count) => count + 1);
dispatcher((action: string) => console.log(action))("go");
export const current: number | null | undefined = holder({ current: 3 }).current;
optimistic((action) => console.log(action))((pending) => pending + 1);
`);
  expect(typeChecked(dir, "use.ts")).toEqual([]);
  writeFileSync(join(dir, "wrong.ts"), `import { keep } from "./lib.js";
const kept = keep({ setValue: (value: number | ((previous: number) => number)) => console.log(value) });
kept.setValue("one");
kept.setValue((count: string) => count);
keep({ setValue: (value: string) => console.log(value) });
`);
  expect(typeChecked(dir, "wrong.ts")).toEqual([expect.stringContaining("error TS2345"), expect.stringContaining("error TS2345"), expect.stringContaining("error TS2322")]);
  const lib = await import(join(dir, "lib.js"));
  const setValue = () => {};
  expect(lib.keep({ setValue }).setValue).toBe(setValue);
  expect(lib.dispatcher(setValue)).toBe(setValue);
});
