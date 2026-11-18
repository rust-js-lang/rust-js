import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { open } from "@rust-js/typescript";
import { runSync } from "./child";
import { fixture, root } from "./support";
import { typeChecked } from "./type-checked";

const declarations = [
  "export declare function keep<const T>(value: T): T;",
  "export declare function plain<T>(value: T): T;",
  'export declare function constrained<const T extends string = "x">(value?: T): T;',
  "export interface Factory {",
  "    <const T>(value: T): T;",
  "    new <const T>(value: T): T;",
  "    keep<const T>(value: T): T;",
  "}",
  "export type Keep = <const T>(value: T) => T;",
].join("\n") + "\n";

const consumer = (module: string) => [
  'import { keep, plain, constrained, type Factory, type Keep } from "./' + module + '.js";',
  // Keep the call separate: contextual return typing can mask a lost const.
  'const result = keep(["a", "b"]);',
  'const exact: readonly ["a", "b"] = result;',
  'const ordinary = plain(["a", "b"]);',
  '// @ts-expect-error a plain parameter must retain its ordinary inference',
  'const widened: readonly ["a", "b"] = ordinary;',
  'const defaulted: "x" = constrained();',
  '// @ts-expect-error the string constraint must survive',
  'constrained(4);',
  'declare const factory: Factory;',
  'const called = factory(["a", "b"]);',
  'const constructed = new factory(["a", "b"]);',
  'const method = factory.keep(["a", "b"]);',
  'const callExact: readonly ["a", "b"] = called;',
  'const newExact: readonly ["a", "b"] = constructed;',
  'const methodExact: readonly ["a", "b"] = method;',
  'declare const functionType: Keep;',
  'const typed = functionType(["a", "b"]);',
  'const typeExact: readonly ["a", "b"] = typed;',
].join("\n") + "\n";

function check(dir: string, module: string) {
  writeFileSync(join(dir, module + "-consumer.ts"), consumer(module));
  const result = typeChecked(dir, module + "-consumer.ts", { types: [], lib: ["es2022"] });
  expect(result.code, result.out).toBe(0);
}

test("const parameter inputs are valid for the original strict consumer", () => {
  const dir = fixture("typescript-const-original");
  writeFileSync(join(dir, "input.d.ts"), declarations);
  check(dir, "input");
});

test("const modifiers survive parameter model and declaration round trips", async () => {
  const dir = fixture("typescript-const-roundtrip");
  const input = join(dir, "input.d.ts");
  writeFileSync(input, declarations);
  const session = await open([input]);
  try {
    const model = await session.read(input);
    const [keep, plain, constrained, factory, functionType] = model.declarations;
    expect(keep).toMatchObject({ typeParameters: [{ name: "T", const: true }] });
    expect(plain).toMatchObject({ typeParameters: [{ name: "T" }] });
    expect("typeParameters" in plain && plain.typeParameters?.[0]).not.toHaveProperty("const");
    expect(constrained).toMatchObject({ typeParameters: [{ name: "T", const: true, constraint: { kind: "keyword", keyword: "string" }, default: { kind: "literal", value: "x" } }] });
    expect(factory).toMatchObject({ members: Array.from({ length: 3 }, () => ({ typeParameters: [{ name: "T", const: true }] })) });
    expect(functionType).toMatchObject({ type: { typeParameters: [{ name: "T", const: true }] } });
    const printed = await session.print(model);
    expect(printed).toContain("keep<const T>(value: T): T;");
    expect(printed).toContain('constrained<const T extends string = "x">');
    const output = join(dir, "output.d.ts");
    writeFileSync(output, printed);
    const reread = await open([output]);
    try { expect(await reread.read(output)).toEqual(model); } finally { await reread.close(); }
    check(dir, "output");
  } finally { await session.close(); }
});

test("printed const parameters preserve literal inference for strict callers", async () => {
  const dir = fixture("typescript-const-callers");
  const input = join(dir, "input.d.ts");
  writeFileSync(input, declarations);
  check(dir, "input");
  const session = await open([input]);
  try {
    writeFileSync(join(dir, "output.d.ts"), await session.print(await session.read(input)));
    check(dir, "output");
  } finally { await session.close(); }
});
