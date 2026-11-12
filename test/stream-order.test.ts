// The order of standard output's and standard error's writes, which the
// corpus, keeping the two streams apart, can't tell: each case here run
// natively and as JS under Node with stderr written into stdout, as a
// terminal shows them, the bytes the same (#1490, ADR 1130). Rust's stdout
// is a `LineWriter`, holding a line till its newline; its stderr is
// unbuffered, each piece written as it's formatted.
import { expect, test } from "bun:test";
import { join } from "node:path";

import { mergedStreams } from "./programs";
import { buildCompiler, fixture, root } from "./support";

// `printed_while_a_line_is_formatted.rs` isn't yet: a stdout line a
// writer's print completes is held with the writer's text, so an
// `eprintln!` it makes after comes first (#1490's rest).
const cases = ["eprinted_while_formatting.rs"];

for (const name of cases) {
  test(`${name}: stdout and stderr interleave as native's`, () => {
    buildCompiler();
    const dir = fixture(`stream-order-${name}`);
    const { native, js } = mergedStreams(join(root, "test/corpus", name), dir);
    expect(js).toBe(native);
  }, 300_000);
}
