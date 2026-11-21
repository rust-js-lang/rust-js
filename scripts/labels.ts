// The labels MIR made in generated JS, `bb5:`, each by the function it's
// in: never one the Rust wrote, a JS label the Rust doesn't have being no
// accepted difference (D1, ADR 0629, amended). Read of the corpus, the
// snapshots and std/ by test/readability.test.ts, and of each crate the
// crate corpus compiles by scripts/crate-corpus.ts: one rule everywhere,
// those not yet structured away listed by name under #2215.

/** Each label MIR made in `text`, the JS of `file`, `<file>: <function>`. */
export function mirLabels(text: string, file: string): string[] {
  const lines = text.split("\n");
  const found: string[] = [];
  lines.forEach((line, at) => {
    if (!/^\s*bb\d+: /.test(line)) return;
    let name = "?";
    for (let up = at; up >= 0; up--) {
      const m = lines[up]!.match(/^(?:export )?(?:async )?function\*? ([\w$]+)\(|^ {2}(?:async )?\*?([\w$]+)\(/);
      if (m) {
        name = m[1] ?? m[2]!;
        break;
      }
    }
    found.push(`${file}: ${name}`);
  });
  return found;
}
