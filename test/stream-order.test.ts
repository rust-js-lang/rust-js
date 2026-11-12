// The order of standard output's and standard error's writes, which the
// corpus, keeping the two streams apart, can't tell: each case here run
// natively and as JS under Node with stderr written into stdout, as a
// terminal shows them, the bytes the same (#1490, ADR 1130). Rust's stdout
// is a `LineWriter`, holding a line till its newline; its stderr is
// unbuffered, each piece written as it's formatted. And as a browser runs
// it, where there's no stream, each stream's lines given its console in
// the order their newlines were written natively.
import { expect, test } from "bun:test";
import { join } from "node:path";

import { mergedStreams } from "./programs";
import { buildCompiler, fixture, root } from "./support";

const cases = [
  "printed_while_a_line_is_formatted.rs",
  "eprinted_while_formatting.rs",
  "stderr_between_a_writers_lines.rs",
  "stderr_inside_a_std_values_debug.rs",
  "line_printed_after_a_part_left_unended.rs",
];

for (const name of cases) {
  test(`${name}: stdout and stderr interleave as native's`, () => {
    buildCompiler();
    const dir = fixture(`stream-order-${name}`);
    const { native, js, nativeLines, browserLines } = mergedStreams(join(root, "test/corpus", name), dir);
    expect(js).toBe(native);
    expect(browserLines).toEqual(nativeLines);
  }, 300_000);
}
