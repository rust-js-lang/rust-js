// `js::shown(value)`'s text, what JS's template makes of it, `${value}`
// (ADR 0312): its `toString`, which may run anything, a JS object's own,
// called here, so a minifier, which takes a template's `toString` to run
// nothing, keeps it in place after what's evaluated before it (#1676).
function $shown(value) {
  return `${value}`;
}
