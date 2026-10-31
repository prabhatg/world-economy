/**
 * Ensemble charts (design system §6): every plausible world as a faint line, the world
 * on the map as one strong line, and the year on the map as a cursor with the value
 * labeled on the mark. Pure SVG, colored by theme tokens, so it reads in light and dark.
 */

import { formatYear } from "./format";

export interface ChartSpec {
  years: number[];
  /** One series per world. */
  series: number[][];
  selected: number;
  /** Index into `years` of the time on the map. */
  t: number;
  log: boolean;
  format: (v: number) => string;
  onSelect?: (world: number) => void;
}

// Drawn at about the panel's width, so 12 px text renders at about 13 px.
const W = 320;
const H = 150;
const PAD = { left: 50, right: 66, top: 24, bottom: 26 };
const NS = "http://www.w3.org/2000/svg";

function el<K extends keyof SVGElementTagNameMap>(name: K, attrs: Record<string, string | number>): SVGElementTagNameMap[K] {
  const e = document.createElementNS(NS, name);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

/** Tick values: powers of ten for a log axis, else 0, half and the top. */
export function ticks(lo: number, hi: number, log: boolean): number[] {
  if (!log) return [lo, (lo + hi) / 2, hi];
  const out: number[] = [];
  for (let p = Math.ceil(Math.log10(lo)); p <= Math.floor(Math.log10(hi)); p++) out.push(10 ** p);
  return out;
}

export function drawChart(svg: SVGSVGElement, spec: ChartSpec): void {
  const { years, series, selected, t, log, format } = spec;
  const values = series.flat().filter((v) => (log ? v > 0 : true));
  let lo = log ? Math.min(...values) : 0;
  let hi = Math.max(...values, log ? 1 : 1e-9);
  if (log) {
    // Fit the data with a little room, then make sure at least one decade tick shows.
    lo = Math.min(lo / 1.5, 10 ** Math.floor(Math.log10(hi)));
    hi = hi * 1.3;
  } else {
    hi = 1;
  }
  const x = (i: number) => PAD.left + ((W - PAD.left - PAD.right) * (years[i] - years[0])) / (years.at(-1)! - years[0]);
  const y = (v: number) => {
    const f = log ? (Math.log10(Math.max(v, lo)) - Math.log10(lo)) / (Math.log10(hi) - Math.log10(lo)) : (v - lo) / (hi - lo);
    return H - PAD.bottom - f * (H - PAD.top - PAD.bottom);
  };
  const path = (s: number[]) => s.map((v, i) => `${i ? "L" : "M"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join("");

  svg.replaceChildren();
  svg.setAttribute("viewBox", `0 0 ${W} ${H}`);
  for (const v of ticks(lo, hi, log)) {
    svg.append(el("line", { x1: PAD.left, x2: W - PAD.right, y1: y(v), y2: y(v), class: "grid" }));
    const label = el("text", { x: PAD.left - 6, y: y(v) + 4, class: "axis", "text-anchor": "end" });
    label.textContent = format(v);
    svg.append(label);
  }
  // Three ticks fit any width; only the last says the era, so they never collide.
  const last = years.length - 1;
  for (const i of [0, Math.round(last / 2), last]) {
    const label = el("text", { x: x(i), y: H - 6, class: "axis", "text-anchor": i === 0 ? "start" : i === last ? "end" : "middle" });
    label.textContent = i === last ? formatYear(years[i]) : formatYear(years[i]).replace(/ (BCE|CE)$/, "");
    svg.append(label);
  }
  series.forEach((s, k) => {
    if (k !== selected) svg.append(el("path", { d: path(s), class: "world" }));
  });
  svg.append(el("path", { d: path(series[selected]), class: "world selected" }));
  // Direct labels at the right end (design system §6: label the marks, no legend).
  const end = (k: number) => y(series[k][last]);
  const mine = end(selected);
  const others = series.map((_, k) => k).filter((k) => k !== selected);
  const lineLabel = (text: string, at: number, cls: string) => {
    const t = el("text", { x: W - PAD.right + 6, y: at + 4, class: cls });
    t.textContent = text;
    svg.append(t);
  };
  lineLabel("This world", mine, "end-label selected");
  if (others.length) {
    // Beside the other world whose end is farthest from this one, so labels never collide.
    const far = others.reduce((a, b) => (Math.abs(end(b) - mine) > Math.abs(end(a) - mine) ? b : a));
    if (Math.abs(end(far) - mine) > 14) lineLabel(`${others.length} others`, end(far), "end-label");
  }
  if (spec.onSelect) {
    series.forEach((s, k) => {
      const hit = el("path", { d: path(s), class: "hit" });
      hit.addEventListener("click", () => spec.onSelect!(k));
      svg.append(hit);
    });
  }
  const cx = x(t);
  const cy = y(series[selected][t]);
  svg.append(el("line", { x1: cx, x2: cx, y1: PAD.top - 6, y2: H - PAD.bottom, class: "cursor" }));
  svg.append(el("circle", { cx, cy, r: 4, class: "dot" }));
  const value = el("text", { x: cx, y: PAD.top - 10, class: "value", "text-anchor": cx > W - 70 ? "end" : cx < PAD.left + 30 ? "start" : "middle" });
  value.textContent = format(series[selected][t]);
  svg.append(value);
}
