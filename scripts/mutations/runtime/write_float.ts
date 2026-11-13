// Mutations of src/runtime/write_float.js (ADR 1135, #1652).
import type { Mutation } from "../../mutations";

const tests = ["test/writer-sink.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "float-zero-pad-sign-unwritten",
    breaks: "a float's `{:08}` into a writer writes no sign first where it has none, where std writes it, `\"\"`, before the zeros",
    file: "src/runtime/write_float.js",
    find: "    f.write(sign);\n",
    replace: "    if (sign !== \"\") f.write(sign);\n",
    tests,
  },
  {
    name: "float-zeros-written-whole",
    breaks: "a float's run of zeros past 64, `1e100`'s, is one `write_str`, where std writes 64 at a time",
    file: "src/runtime/write_float.js",
    find: "    while (zeros > 64) {\n",
    replace: "    while (false) {\n",
    tests,
  },
  {
    name: "float-exact-digits-unbounded",
    breaks: "a float's `{:.30}` writes every digit as one, where std makes at most `estimate_max_buf_len` of them and writes the rest as zeros",
    file: "src/runtime/write_float.js",
    find: "    const fixed = $floatFixedDigits(size, precision, $floatMaxDigits(size, f32));\n",
    replace: "    const fixed = $floatFixedDigits(size, precision, Infinity);\n",
    tests,
  },
  {
    name: "float-power-of-two-decoded-as-any",
    breaks: "a float whose significand is a power of two is decoded one exponent too high, so `{:.60}` of `0.125_f32` makes one digit fewer than std",
    file: "src/runtime/write_float.js",
    find: "  const decoded = !normal ? exponent2 : power ? exponent2 - 2 : exponent2 - 1;\n",
    replace: "  const decoded = !normal ? exponent2 : exponent2 - 1;\n",
    tests,
  },
  {
    name: "float-fixed-tie-up",
    breaks: "a float's `{:.0}` of `2.5` rounds a tie up, `3`, where std rounds it to even, `2`",
    file: "src/runtime/write_float.js",
    find: "  if (twice > denominator || (twice === denominator && digits % 2n === 1n)) digits += 1n;\n",
    replace: "  if (twice >= denominator) digits += 1n;\n",
    tests,
  },
  {
    name: "float-debug-exponent-past-1e16",
    breaks: "a float's `{:?}` of `1e16` is written in decimal, where std writes `1e16`",
    file: "src/runtime/write_float.js",
    find: '  if (trait === "G" && size !== 0 && (size >= 1e16 || size < tiny)) {\n',
    replace: '  if (trait === "G" && size !== 0 && (size > 1e16 || size < tiny)) {\n',
    tests,
  },
];
