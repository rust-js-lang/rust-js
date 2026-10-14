// What a `bun test` run says of itself, read one way, by the scripts that
// run one and judge it: the qualification's suites and the mutations'.

/** How many tests passed, failed and were skipped, by the run's summary. */
export function summary(output: string): { pass: number; fail: number; skip: number } {
  const count = (what: string) => Number(new RegExp(String.raw`^ (\d+) ` + what + "$", "m").exec(output)?.[1] ?? 0);
  return { pass: count("pass"), fail: count("fail"), skip: count("skip") };
}

/** Whether a run passed: it ended on its own, `ended`, with code 0, some
 * test passing and none failing. */
export function passed(code: number | null, ended: boolean, output: string): boolean {
  const { pass, fail } = summary(output);
  return code === 0 && ended && pass > 0 && fail === 0;
}
