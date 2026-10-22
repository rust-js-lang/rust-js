// A fence's code, highlighted at build time, as spans of classes the site's
// CSS colors: `k` keyword, `s` string, `c` comment, `n` number, `f` function,
// `t` type, `m` macro, `a` attribute, `l` lifetime, `g` JSX tag, `p` property
// key. One tokenizer, told each language's words and quotes; what it doesn't
// know is plain text, escaped.

import { escapeHtml } from "./markdown";

interface Language {
  keywords: Set<string>;
  /** Literal words: `true`, `null`, `None`. */
  constants: Set<string>;
  /** Types named in lowercase: Rust's `u32`. */
  primitives?: Set<string>;
  lineComment: string[];
  blockComment?: [string, string];
  quotes: string[];
  /** JS's template literals. */
  template?: boolean;
  /** Rust's: `'a` lifetimes, `r#".."#` raw strings, `name!` macros, `#[..]`. */
  rust?: boolean;
  /** JSX's `<Tag` and `</Tag>`. */
  jsx?: boolean;
  /** TOML's `[table]` and `key =`, JSON's `"key":`. */
  keys?: boolean;
}

const words = (s: string) => new Set(s.split(/\s+/).filter(Boolean));

const rust: Language = {
  keywords: words(`as async await break const continue crate dyn else enum extern fn for if impl in let loop match mod move mut
    pub ref return self Self static struct super trait type unsafe use where while`),
  constants: words("true false None Some Ok Err"),
  primitives: words("bool char str u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64"),
  lineComment: ["//"],
  blockComment: ["/*", "*/"],
  quotes: ['"'],
  rust: true,
};

const js: Language = {
  keywords: words(`async await break case catch class const continue default delete do else export extends finally for from
    function if import in instanceof let new of return static super switch this throw try typeof var void while yield as type
    interface`),
  constants: words("true false null undefined NaN Infinity"),
  lineComment: ["//"],
  blockComment: ["/*", "*/"],
  quotes: ['"', "'"],
  template: true,
};

const languages: Record<string, Language> = {
  rust,
  rs: rust,
  js,
  javascript: js,
  ts: js,
  typescript: js,
  jsx: { ...js, jsx: true },
  tsx: { ...js, jsx: true },
  json: { keywords: words(""), constants: words("true false null"), lineComment: [], quotes: ['"'], keys: true },
  toml: { keywords: words(""), constants: words("true false"), lineComment: ["#"], quotes: ['"', "'"], keys: true },
  bash: {
    keywords: words("if then else fi for do done case esac export cd echo"),
    constants: words(""),
    lineComment: ["#"],
    quotes: ['"', "'"],
  },
};
languages.sh = languages.bash;
languages.shell = languages.bash;

const span = (cls: string, text: string) => `<span class="${cls}">${escapeHtml(text)}</span>`;

export function highlight(code: string, lang: string): string {
  const language = languages[lang];
  if (!language) return escapeHtml(code);
  let out = "";
  let k = 0;
  // Whether a `<` starts a JSX tag: after what can't end an operand.
  let operandEnded = false;
  while (k < code.length) {
    const rest = code.slice(k);
    const c = code[k];
    const comment = language.lineComment.find((m) => rest.startsWith(m));
    if (comment && (comment !== "#" || k === 0 || /\s/.test(code[k - 1]))) {
      const end = code.indexOf("\n", k);
      const stop = end < 0 ? code.length : end;
      out += span("c", code.slice(k, stop));
      k = stop;
      continue;
    }
    if (language.blockComment && rest.startsWith(language.blockComment[0])) {
      const end = code.indexOf(language.blockComment[1], k + 2);
      const stop = end < 0 ? code.length : end + language.blockComment[1].length;
      out += span("c", code.slice(k, stop));
      k = stop;
      continue;
    }
    if (language.rust && c === "#" && (rest.startsWith("#[") || rest.startsWith("#!["))) {
      const end = matchingBracket(code, code.indexOf("[", k));
      out += span("a", code.slice(k, end + 1));
      k = end + 1;
      continue;
    }
    if (language.rust && /^b?r#*"/.test(rest)) {
      const open = /^b?r(#*)"/.exec(rest)!;
      const close = `"${open[1]}`;
      const end = code.indexOf(close, k + open[0].length);
      const stop = end < 0 ? code.length : end + close.length;
      out += span("s", code.slice(k, stop));
      k = stop;
      operandEnded = true;
      continue;
    }
    if (language.rust && c === "'") {
      // `'a'` is a char, `'\n'` too; `'a` alone a lifetime.
      const char = /^'(\\.|[^\\'])'/.exec(rest) ?? /^'\\u\{[0-9a-fA-F]+\}'/.exec(rest);
      if (char) {
        out += span("s", char[0]);
        k += char[0].length;
        operandEnded = true;
        continue;
      }
      const lifetime = /^'[A-Za-z_]\w*/.exec(rest);
      if (lifetime) {
        out += span("l", lifetime[0]);
        k += lifetime[0].length;
        continue;
      }
    }
    if (language.quotes.includes(c) || (language.template && c === "`")) {
      const end = stringEnd(code, k, c);
      const literal = code.slice(k, end);
      const key = language.keys && /^\s*:/.test(code.slice(end));
      out += span(key ? "p" : "s", literal);
      k = end;
      operandEnded = true;
      continue;
    }
    if (language.jsx && c === "<" && !operandEnded) {
      const tag = /^<\/?([A-Za-z][\w.]*)/.exec(rest);
      if (tag) {
        out += escapeHtml(tag[0].slice(0, tag[0].length - tag[1].length)) + span("g", tag[1]);
        k += tag[0].length;
        continue;
      }
    }
    if (language.keys && c === "[" && (k === 0 || code[k - 1] === "\n")) {
      const end = code.indexOf("]", k);
      if (end > 0 && !code.slice(k, end).includes("\n")) {
        out += span("t", code.slice(k, end + 1));
        k = end + 1;
        continue;
      }
    }
    const number = /^(0x[0-9a-fA-F_]+|0b[01_]+|\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d+)?)([iu](8|16|32|64|128|size)|f32|f64|n)?/.exec(rest);
    if (number && (k === 0 || !/[\w$]/.test(code[k - 1]))) {
      out += span("n", number[0]);
      k += number[0].length;
      operandEnded = true;
      continue;
    }
    const word = /^[A-Za-z_$][\w$]*/.exec(rest);
    if (word) {
      const w = word[0];
      const after = code.slice(k + w.length);
      let cls = "";
      if (language.keys && /^\s*=/.test(after) && /(^|\n)\s*$/.test(code.slice(0, k))) cls = "p";
      else if (language.keywords.has(w)) cls = "k";
      else if (language.constants.has(w)) cls = "n";
      else if (language.primitives?.has(w)) cls = "t";
      else if (language.rust && after.startsWith("!") && !after.startsWith("!=")) cls = "m";
      else if (/^\s*(::\s*<[^>]*>\s*)?\(/.test(after) && code[k - 1] !== ".") cls = "f";
      else if (/^\s*\(/.test(after)) cls = "f";
      else if (/^[A-Z]/.test(w)) cls = "t";
      out += cls ? span(cls, w) + (cls === "m" ? span("m", "!") : "") : escapeHtml(w);
      k += w.length + (cls === "m" ? 1 : 0);
      operandEnded = !language.keywords.has(w) || w === "this" || w === "self";
      continue;
    }
    out += escapeHtml(c);
    if (!/\s/.test(c)) operandEnded = /[)\]}]/.test(c);
    k++;
  }
  return out;
}

/** The index after the string that opens at `start` with `quote`. */
function stringEnd(code: string, start: number, quote: string): number {
  let k = start + 1;
  while (k < code.length) {
    if (code[k] === "\\") {
      k += 2;
      continue;
    }
    if (code[k] === quote) return k + 1;
    if (code[k] === "\n" && quote !== "`" && quote !== '"') return k;
    k++;
  }
  return code.length;
}

function matchingBracket(code: string, open: number): number {
  let depth = 0;
  for (let k = open; k < code.length; k++) {
    if (code[k] === "[") depth++;
    if (code[k] === "]" && --depth === 0) return k;
  }
  return code.length - 1;
}
