// The rust-js guide's site (ADR 0600): the theme, the menu on a small
// screen, a copy button on each code block, the section you're reading in
// "On this page", and search over every page's sections, `search.json`.

const root = document.documentElement;
const base = document.body.dataset.base || ".";

// ── Theme ─────────────────────────────────────────────────────────────
document.querySelector(".theme")?.addEventListener("click", () => {
  const theme = root.dataset.theme === "dark" ? "light" : "dark";
  root.dataset.theme = theme;
  localStorage.setItem("theme", theme);
});

// ── Menu ──────────────────────────────────────────────────────────────
const menu = document.querySelector(".menu");
const setMenu = (open) => {
  document.body.classList.toggle("nav-open", open);
  menu?.setAttribute("aria-expanded", String(open));
};
menu?.addEventListener("click", () => setMenu(!document.body.classList.contains("nav-open")));
document.querySelector(".sidebar")?.addEventListener("click", (e) => {
  if (e.target.closest("a")) setMenu(false);
});
document.querySelector(".content")?.addEventListener("click", () => setMenu(false));
document.querySelector(".sidebar [aria-current]")?.scrollIntoView({ block: "center" });

// ── Copy ──────────────────────────────────────────────────────────────
for (const block of document.querySelectorAll(".code")) {
  const button = document.createElement("button");
  button.className = "copy";
  button.type = "button";
  button.textContent = "Copy";
  button.addEventListener("click", async () => {
    await navigator.clipboard.writeText(block.querySelector("code").textContent);
    button.textContent = "Copied";
    button.classList.add("done");
    setTimeout(() => {
      button.textContent = "Copy";
      button.classList.remove("done");
    }, 1500);
  });
  block.append(button);
}

// ── On this page ──────────────────────────────────────────────────────
const tocLinks = [...document.querySelectorAll(".toc a")];
if (tocLinks.length > 0) {
  const targets = tocLinks.map((a) => document.getElementById(decodeURIComponent(a.hash.slice(1)))).filter(Boolean);
  const mark = () => {
    const line = (parseFloat(getComputedStyle(root).scrollPaddingTop) || 0) + 8;
    let current = targets[0];
    for (const target of targets) if (target.getBoundingClientRect().top <= line) current = target;
    for (const a of tocLinks) a.classList.toggle("active", a.hash.slice(1) === current?.id);
  };
  addEventListener("scroll", mark, { passive: true });
  mark();
}

// ── Search ────────────────────────────────────────────────────────────
const dialog = document.querySelector(".search");
const input = dialog.querySelector("input");
const results = dialog.querySelector(".results");
let index;
let selected = 0;

async function loadIndex() {
  if (!index) index = await (await fetch(`${base}/search.json`)).json();
  return index;
}

function openSearch() {
  dialog.hidden = false;
  input.focus();
  input.select();
  loadIndex().then(() => show(input.value));
}

function closeSearch() {
  dialog.hidden = true;
}

const escapeHtml = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const escapeRegExp = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

function score(entry, terms) {
  const heading = `${entry.page} ${entry.heading}`.toLowerCase();
  const text = entry.text.toLowerCase();
  let total = 0;
  for (const term of terms) {
    const inHeading = heading.includes(term);
    const inText = text.includes(term);
    if (!inHeading && !inText) return 0;
    total += (inHeading ? 10 : 0) + (inText ? 1 : 0) + (entry.heading.toLowerCase().startsWith(term) ? 5 : 0);
  }
  return total;
}

function snippet(text, terms) {
  const lower = text.toLowerCase();
  const at = Math.max(0, Math.min(...terms.map((t) => lower.indexOf(t)).filter((i) => i >= 0)) - 40);
  const part = (at > 0 ? "…" : "") + text.slice(at, at + 160) + (at + 160 < text.length ? "…" : "");
  const pattern = new RegExp(`(${terms.map(escapeRegExp).join("|")})`, "gi");
  return escapeHtml(part).replace(pattern, "<mark>$1</mark>");
}

function show(query) {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  selected = 0;
  if (terms.length === 0 || !index) {
    results.innerHTML = "";
    return;
  }
  const found = index
    .map((entry) => ({ entry, score: score(entry, terms) }))
    .filter((r) => r.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, 12);
  if (found.length === 0) {
    results.innerHTML = `<li class="none">Nothing in the guide matches “${escapeHtml(query)}”.</li>`;
    return;
  }
  results.innerHTML = found
    .map(
      ({ entry }, k) =>
        `<li role="option" aria-selected="${k === 0}"><a href="${base}/${entry.url}">` +
        `<span class="where">${escapeHtml(entry.page)}</span>` +
        `<span class="what">${escapeHtml(entry.heading || entry.page)}</span>` +
        `<span class="snippet">${snippet(entry.text, terms)}</span></a></li>`,
    )
    .join("");
}

function move(by) {
  const options = [...results.querySelectorAll("[role=option]")];
  if (options.length === 0) return;
  selected = (selected + by + options.length) % options.length;
  options.forEach((o, k) => o.setAttribute("aria-selected", String(k === selected)));
  options[selected].scrollIntoView({ block: "nearest" });
}

input.addEventListener("input", () => show(input.value));
input.addEventListener("keydown", (e) => {
  if (e.key === "ArrowDown") {
    e.preventDefault();
    move(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    move(-1);
  } else if (e.key === "Enter") {
    results.querySelector("[aria-selected=true] a")?.click();
  }
});
results.addEventListener("click", (e) => {
  if (e.target.closest("a")) closeSearch();
});
dialog.addEventListener("click", (e) => {
  if (e.target === dialog) closeSearch();
});
document.querySelector(".search-open")?.addEventListener("click", openSearch);
addEventListener("keydown", (e) => {
  const typing = /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName);
  if ((e.key === "/" && !typing) || (e.key === "k" && (e.metaKey || e.ctrlKey))) {
    e.preventDefault();
    openSearch();
  } else if (e.key === "Escape" && !dialog.hidden) {
    closeSearch();
  }
});
