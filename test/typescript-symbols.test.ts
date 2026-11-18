import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { memberKey, open } from "@rust-js/typescript";
import { fixture, root } from "./support";
import { runSync } from "./child";
import { typeChecked } from "./type-checked";

test("computed symbol members keep their declaration keys through the model", async () => {
  const file = join(fixture("typescript-symbol-keys"), "symbols.d.ts");
  writeFileSync(file, "export interface Symbols {\n    [Symbol.iterator](): Iterator<string>;\n    readonly [Symbol.toStringTag]: string;\n    [Symbol.dispose]?(): void;\n}\n");
  const ts = await open([file]);
  try {
    const model = await ts.read(file);
    const declaration = model.declarations[0];
    if (declaration.kind !== "interface") throw new Error("interface expected");
    expect(declaration.members.map(member => "name" in member ? member.name : undefined)).toEqual([
      { computed: "Symbol.iterator" }, { computed: "Symbol.toStringTag" }, { computed: "Symbol.dispose" },
    ]);
    const printed = await ts.print(model);
    expect(printed).toContain("[Symbol.iterator](): Iterator<string>");
    expect(printed).toContain("readonly [Symbol.toStringTag]: string");
    expect(printed).toContain("[Symbol.dispose]?(): void");
    expect(printed).not.toContain("\"[Symbol.iterator]\"");
    const again = join(fixture("typescript-symbol-roundtrip"), "symbols.d.ts");
    writeFileSync(again, printed);
    const reread = await open([again]);
    try { expect(await reread.read(again)).toEqual(model); } finally { await reread.close(); }
  } finally { await ts.close(); }
});


test("computed symbol declarations remain usable by strict TypeScript consumers", async () => {
  const dir = fixture("typescript-symbol-consumer");
  const input = join(dir, "input.d.ts");
  writeFileSync(input, 'export interface Symbols { [Symbol.iterator](): Iterator<string>; readonly [Symbol.toStringTag]: string; [Symbol.dispose]?(): void; "[Symbol.iterator]"(): number; }\n');
  const ts = await open([input]);
  try {
    const model = await ts.read(input);
    const declaration = model.declarations[0];
    if (declaration.kind !== "interface") throw new Error("interface expected");
    expect(declaration.members.at(-1)).toMatchObject({name:"[Symbol.iterator]"});
    expect(memberKey("font-family")).toBe("font-family");
    expect(memberKey(JSON.stringify("[Symbol.iterator]"))).toBe(JSON.stringify(JSON.stringify("[Symbol.iterator]")));
    expect(declaration.members.map(member => "name" in member ? memberKey(member.name) : undefined)).toEqual(["[Symbol.iterator]", "[Symbol.toStringTag]", "[Symbol.dispose]", JSON.stringify("[Symbol.iterator]")]);
    writeFileSync(join(dir,"output.d.ts"), await ts.print(model));
    const consumer = (module: string) => 'import type { Symbols } from "./'+module+'";\ndeclare const value: Symbols;\nconst iterable: Iterable<string> = value;\nconst brand: string = value[Symbol.toStringTag];\nvalue[Symbol.dispose]?.();\nconst other: number = value["[Symbol.iterator]"]();\n';
    for (const module of ["input","output"]) {
      writeFileSync(join(dir,module+"-consumer.ts"),consumer(module+".d.ts"));
      const checked = typeChecked(dir, module+"-consumer.ts", { types: [], lib: ["esnext"] });
      expect(checked.code,checked.out).toBe(0);
    }
  } finally { await ts.close(); }
});

test("computed member expressions outside symbol paths are refused explicitly", async () => {
  const file = join(fixture("typescript-computed-expression"),"input.d.ts");
  for (const expression of ['Symbol["iterator"]', 'Symbol?.iterator']) {
    writeFileSync(file, 'export interface Symbols { [' + expression + '](): Iterator<string>; }\n');
    const ts = await open([file]);
    try {
      const model = await ts.read(file);
      await expect(ts.print(model)).rejects.toThrow("computed member name is not a symbol path");
    } finally { await ts.close(); }
  }
});
