# node: Node.js for rust-js

Node's modules for a program's Rust that runs in Node, typed as
`@types/node` types them, each the JS you'd write
([ADR 0272](../docs/decisions/0272-node.md)):

```rust
use node::{BufferEncoding, fs, process};

let root = process::cwd() + "/src/content";
let text = fs::read_file_sync(&(root + "/index.md"), BufferEncoding::Utf8);
```

```js
import { readFileSync } from "fs";

const root = process.cwd() + "/src/content";
const text = readFileSync(root + "/index.md", "utf8");
```

What it has is what ports have needed: `fs::read_file_sync` and
`process::cwd`. And what `@types/node` adds to `Error`, V8's stack trace API,
`js_error::prepare_stack_trace()`, `Error.prepareStackTrace`, which
Chrome has too ([ADR 0648](../docs/decisions/0648-v8s-stack-trace-api-is-the-node-crates.md)).
