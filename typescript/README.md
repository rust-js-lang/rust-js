# @rust-js/typescript

TypeScript's own parser and printer, through TypeScript 7's API, and a JSON
model of declarations between them: what rust-js reads TypeScript with, and
writes its `.d.ts` with. See
[ADR 0206](../docs/decisions/0206-typescript-module.md).

```js
import { open } from "@rust-js/typescript";

const ts = await open(["index.d.ts"]);
const { declarations } = await ts.read("index.d.ts");
console.log(await ts.print({ declarations }));
await ts.close();
```
