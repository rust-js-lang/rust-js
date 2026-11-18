import { expect, test } from "bun:test";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { open } from "@rust-js/typescript";
import { runSync } from "./child";
import { fixture, root } from "./support";
import { typeChecked } from "./type-checked";

const declarations = "export interface Box<T = string> {\n    value: T;\n    map<U extends T = T>(value: U): U;\n}\n\nexport type List<T = number> = readonly T[];\n\nexport declare function pick<T = string>(value?: T): T;\n\nexport interface Factory {\n    new <T = object>(): Box<T>;\n}\n\nexport interface Bounded<T extends string = \"x\"> {\n    value: T;\n}\n\nexport type Maker = <T = number>() => T;\n";

test("reading generic defaults preserves them in every parameter-bearing declaration", async () => {
  const dir = fixture("typescript-default-reader");
  const input = join(dir, "input.d.ts");
  writeFileSync(input, declarations);
  const session = await open([input]);
  try {
    const model = await session.read(input);
    const [box, list, pick, factory, bounded, maker] = model.declarations;
    expect(box).toMatchObject({typeParameters:[{name:"T",default:{kind:"keyword",keyword:"string"}}]});
    expect(box).toMatchObject({members:[{}, {kind:"method",typeParameters:[{name:"U",constraint:{kind:"reference",name:"T",args:[]},default:{kind:"reference",name:"T",args:[]}}]}]});
    expect(list).toMatchObject({typeParameters:[{name:"T",default:{kind:"keyword",keyword:"number"}}]});
    expect(pick).toMatchObject({typeParameters:[{name:"T",default:{kind:"keyword",keyword:"string"}}]});
    expect(factory).toMatchObject({members:[{kind:"construct",typeParameters:[{name:"T",default:{kind:"keyword",keyword:"object"}}]}]});
    expect(bounded).toMatchObject({typeParameters:[{name:"T",constraint:{kind:"keyword",keyword:"string"},default:{kind:"literal",value:"x"}}]});
    expect(maker).toMatchObject({type:{kind:"function",typeParameters:[{name:"T",default:{kind:"keyword",keyword:"number"}}]}});
    const printed = await session.print(model);
    expect(printed).toBe(declarations);
    const output = join(dir,"output.d.ts");
    writeFileSync(output, printed);
    const again = await open([output]);
    try { expect(await again.read(output)).toEqual(model); } finally { await again.close(); }
  } finally { await session.close(); }
});

test("original and printed defaults accept strict callers that omit type arguments", async () => {
  const dir = fixture("typescript-default-callers");
  const input = join(dir,"input.d.ts");
  writeFileSync(input,declarations);
  const session = await open([input]);
  try {
    writeFileSync(join(dir,"output.d.ts"),await session.print(await session.read(input)));
    for (const module of ["input","output"]) {
      const consumer=module+"-consumer.ts";
      writeFileSync(join(dir,consumer),'import {pick,type Box,type List,type Factory,type Bounded,type Maker} from "./'+module+'.js";\ndeclare const box: Box;\nconst value: string = box.value;\nconst selected: string = box.map("x");\nconst list: List = [1,2];\nconst text: string = pick();\ndeclare const factory: Factory;\nconst made: Box<object> = new factory();\ndeclare const bounded: Bounded;\nconst literal: \"x\" = bounded.value;\n// @ts-expect-error the constraint is preserved\ntype WrongBound = Bounded<number>;\n// @ts-expect-error the literal default is preserved\nconst wrongLiteral: \"y\" = bounded.value;\ndeclare const make: Maker;\nconst madeNumber: number = make();\n');
      const result = typeChecked(dir, consumer, { types: [], lib: ["es2022"] });
      expect(result.code,result.out).toBe(0);
    }
  } finally { await session.close(); }
});
