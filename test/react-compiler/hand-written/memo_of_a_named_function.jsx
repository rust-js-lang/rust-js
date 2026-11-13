// A memo given a function named before it, as a person may write it:
// React Compiler refuses it, "Expected the first argument to be an inline
// function expression", as it does rust-js's of the same Rust,
// `use_memo(report, [n])`.
import { useMemo, useState } from "react";

export function Ticker() {
  const [n] = useState(0);
  const report = () => n;
  const reported = useMemo(report, [n]);
  return <p>{reported}</p>;
}
