// What react/generate.ts and react/attributes.ts both name Rust's way:
// each event handler's event, and a JS name as a Rust one.

// Which React event each handler gets, as react.dev's common components
// page groups them, by @types/react's names. Any other is `SyntheticEvent`,
// React's base event.
export const EVENT_TYPES: Record<string, string> = {};
const groups: Record<string, string> = {
  AnimationEvent: "AnimationEnd AnimationIteration AnimationStart",
  MouseEvent: "AuxClick Click ContextMenu DoubleClick MouseDown MouseEnter MouseLeave MouseMove MouseOut MouseOver MouseUp",
  InputEvent: "BeforeInput",
  FocusEvent: "Blur Focus",
  CompositionEvent: "CompositionEnd CompositionStart CompositionUpdate",
  ClipboardEvent: "Copy Cut Paste",
  DragEvent: "Drag DragEnd DragEnter DragExit DragLeave DragOver DragStart Drop",
  PointerEvent:
    "GotPointerCapture LostPointerCapture PointerCancel PointerDown PointerEnter PointerLeave PointerMove PointerOut PointerOver PointerUp",
  KeyboardEvent: "KeyDown KeyPress KeyUp",
  TouchEvent: "TouchCancel TouchEnd TouchMove TouchStart",
  TransitionEvent: "TransitionCancel TransitionEnd TransitionRun TransitionStart",
  WheelEvent: "Wheel",
  ToggleEvent: "BeforeToggle Toggle",
  ChangeEvent: "Change Input",
  UIEvent: "Scroll ScrollEnd",
};
for (const [type, names] of Object.entries(groups)) for (const name of names.split(" ")) EVENT_TYPES[`on${name}`] = type;

const KEYWORDS = new Set(
  "as async await box break const continue crate default do dyn else enum extern false final fn for gen if impl in let loop macro match mod move mut override priv pub ref return self static struct super trait true try type typeof unsafe unsized use virtual where while yield".split(
    " ",
  ),
);

export function snake(name: string): string {
  const s = name
    .replace(/[-:]/g, "_")
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/([A-Z])([A-Z][a-z])/g, "$1_$2")
    .toLowerCase();
  return KEYWORDS.has(s) ? `r#${s}` : s;
}
