import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { checkedMembers, open } from "@rust-js/typescript";
import { runSync } from "./child";
import { fixture, root } from "./support";

const declarations = [
  "export interface Callable {",
  "    <T extends object = { value: number }>(value: T, label?: string, ...extra: number[]): T;",
  "    (): undefined;",
  "    new (value: number): { value: number };",
  "    call(value: string): string;",
  "    [Symbol.iterator](): Iterator<number>;",
  "}",
  "export type Nested = { <T>(value: T): T; (value: number, extra?: boolean): number };",
].join("\n") + "\n";

function check(dir: string, module: string) {
  writeFileSync(join(dir, module + "-consumer.ts"), [
    'import type { Callable, Nested } from "./' + module + '.js";',
    "declare const callable: Callable;",
    "const result: { value: number } = callable({ value: 1 }, undefined, 1, 2);",
    "const empty: undefined = callable();",
    "const made: { value: number } = new callable(4);",
    'const method: string = callable.call("x");',
    "const iterable: Iterable<number> = callable;",
    "// @ts-expect-error a required generic object is not an optional argument",
    "callable(undefined);",
    "// @ts-expect-error rest elements retain their numeric type",
    'callable({ value: 1 }, "x", "wrong");',
    "declare const nested: Nested;",
    'const text: string = nested("x");',
    "const number: number = nested(1, true);",
  ].join("\n") + "\n");
  const config = join(dir, module + ".json");
  writeFileSync(config, JSON.stringify({ compilerOptions: { strict: true, noEmit: true, types: [], lib: ["esnext"], module: "esnext", moduleResolution: "bundler" }, files: [module + "-consumer.ts"] }));
  const result = runSync([process.execPath, join(root, "node_modules/typescript/bin/tsc"), "-p", config], dir, 10_000);
  expect(result.code, result.stdout + result.stderr).toBe(0);
}

test("original callable interface and object signatures pass strict consumers", () => {
  const dir = fixture("typescript-call-original");
  writeFileSync(join(dir, "input.d.ts"), declarations);
  check(dir, "input");
});

test("call members preserve overloads and stay distinct from constructs and named methods", async () => {
  const dir = fixture("typescript-call-model");
  const input = join(dir, "input.d.ts");
  writeFileSync(input, declarations);
  const session = await open([input]);
  try {
    const model = await session.read(input);
    const declaration = model.declarations[0];
    if (declaration.kind !== "interface") throw new Error("interface expected");
    expect(checkedMembers(declaration.members)).toBe(declaration.members);
    expect(declaration.members.map(m => m.kind)).toEqual(["call", "call", "construct", "method", "method"]);
    expect(declaration.members[0]).toMatchObject({ kind: "call", typeParameters: [{ name: "T", constraint: { kind: "keyword", keyword: "object" }, default: { kind: "object" } }], params: [{ name: "value", optional: false, rest: false }, { name: "label", optional: true }, { name: "extra", rest: true, type: { kind: "array", element: { kind: "keyword", keyword: "number" } } }], returns: { kind: "reference", name: "T" } });
    expect(model.declarations[1]).toMatchObject({ type: { kind: "object", members: [{ kind: "call", typeParameters: [{ name: "T" }] }, { kind: "call", params: [{ name: "value" }, { name: "extra", optional: true }] }] } });
    const printed = await session.print(model);
    expect(printed).toContain("<T extends object =");
    expect(printed).toContain("(): undefined;");
    writeFileSync(join(dir, "output.d.ts"), printed);
    const again = await open([join(dir, "output.d.ts")]);
    try { expect(await again.read(join(dir, "output.d.ts"))).toEqual(model); } finally { await again.close(); }
    check(dir, "output");
  } finally { await session.close(); }
});
