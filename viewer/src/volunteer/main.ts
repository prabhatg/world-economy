/**
 * ManyWorlds browser client v0 (#22, ADR-0100): lends this tab to the coordinator that
 * serves it. Each world runs in a Web Worker on the engine the gate checks (the wasm
 * CLI); the page shows what it is doing, the world it last computed and what became of
 * it. Nothing runs until the volunteer presses Start (design system: respect autonomy).
 */

import "../../../design/tokens.css";
import "../styles.css";
import "./volunteer.css";
import { drawChart } from "../chart";
import { formatPeople, formatYear } from "../format";
import type { WorkResult } from "./engine";
import { type Done, type Engine, Host, type PackStore, Stopped } from "./host";
import { type State, describe, verdict } from "./status";
import type { FromWorker, ToWorker } from "./worker";

const RETRY_MS = 60_000;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const ui = {
  toggle: $<HTMLButtonElement>("toggle"),
  battery: $<HTMLButtonElement>("battery"),
  announce: $("now-announce"),
  headline: $("now-headline"),
  detail: $("now-detail"),
  progress: $<HTMLProgressElement>("progress"),
  yours: $("yours"),
  caption: $("yours-caption"),
  chart: document.getElementById("yours-chart") as unknown as SVGSVGElement,
  verdict: $("verdict"),
  worlds: $("worlds"),
  years: $("years"),
  host: $("host"),
};

/** Reads and writes that may fail (private windows, blocked storage) without breaking the page. */
const stored = {
  get: (k: string) => {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  },
  set: (k: string, v: string) => {
    try {
      localStorage.setItem(k, v);
    } catch {
      /* counted for this visit only */
    }
  },
};

/** This browser's name to the coordinator: random, kept so its record of agreement builds up. */
function hostName(): string {
  let name = stored.get("oikos-host");
  if (!name) {
    const bytes = crypto.getRandomValues(new Uint8Array(6));
    name = `browser-${[...bytes].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
    stored.set("oikos-host", name);
  }
  return name;
}

/** Packs kept with the Cache API where the page may use it (https or localhost), else for this visit. */
class BrowserPacks implements PackStore {
  private memory = new Map<string, Uint8Array>();
  private cache = "caches" in globalThis ? caches.open("oikos-packs-v1").catch(() => null) : Promise.resolve(null);
  async get(id: string) {
    const c = await this.cache;
    const hit = c && (await c.match(`packs/${id}`));
    return hit ? new Uint8Array(await hit.arrayBuffer()) : this.memory.get(id);
  }
  async put(id: string, bytes: Uint8Array) {
    const c = await this.cache;
    if (c) await c.put(`packs/${id}`, new Response(bytes as BodyInit)).catch(() => this.memory.set(id, bytes));
    else this.memory.set(id, bytes);
  }
}

/** Ends the run in progress, if any: its worker and the promise waiting on it. */
let stopRun: (() => void) | null = null;
/** Aborts the round in progress: its downloads, uploads and reports. */
let round: AbortController | null = null;

/** The engine in a fresh Web Worker per run. */
function workerEngine(module: WebAssembly.Module): Engine {
  return (unit, packs, onProgress) =>
    new Promise((resolve, reject) => {
      const w = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
      const end = () => {
        w.terminate();
        stopRun = null;
      };
      stopRun = () => {
        end();
        reject(new Stopped());
      };
      w.onmessage = (e: MessageEvent<FromWorker>) => {
        const m = e.data;
        if (m.kind === "progress") onProgress?.(m.years);
        else {
          end();
          if (m.kind === "done") resolve({ result: m.result as WorkResult, files: m.files });
          else reject(new Error(m.message));
        }
      };
      w.onerror = (e) => {
        end();
        reject(new Error(e.message || "the engine stopped"));
      };
      w.postMessage({ module, unit, packs } satisfies ToWorker);
    });
}

/** Each Start begins a new generation; a loop ends when its generation is no longer current. */
let generation = 0;
let running = false;
let runStarted = 0;
let wake: (() => void) | null = null;

let announced = "";

function show(s: State): void {
  const line = describe(s, s.stage === "run" ? performance.now() - runStarted : 0);
  ui.headline.textContent = line.headline;
  ui.detail.textContent = line.detail;
  ui.progress.hidden = line.progress === null;
  if (line.progress !== null) {
    ui.progress.value = line.progress;
    ui.progress.setAttribute("aria-valuetext", line.detail);
  }
  ui.battery.hidden = s.stage !== "battery";
  // Screen readers hear each stage once, not every century of a run.
  if (s.stage !== announced) {
    announced = s.stage;
    ui.announce.textContent = s.stage === "run" ? "Simulating a world" : line.headline;
  }
  if (s.stage === "error") console.warn("ManyWorlds:", s.message);
}

function tally(add?: { years: number }): void {
  let worlds = Number(stored.get("oikos-worlds") ?? 0);
  let years = Number(stored.get("oikos-years") ?? 0);
  if (add) {
    worlds += 1;
    years += add.years;
    stored.set("oikos-worlds", String(worlds));
    stored.set("oikos-years", String(years));
  }
  const n = new Intl.NumberFormat("en-US");
  ui.worlds.textContent = n.format(worlds);
  ui.years.textContent = n.format(years);
}

function showWorld(done: Done): void {
  const summaries = done.run.result.summaries ?? [];
  if (summaries.length < 2) return;
  const years = summaries.map((s) => s.calendar_year);
  const people = summaries.map((s) => s.population);
  ui.yours.hidden = false;
  const story =
    `People in your world went from ${formatPeople(people[0])} to ${formatPeople(people.at(-1)!)} ` +
    `between ${formatYear(years[0])} and ${formatYear(years.at(-1)!)}`;
  ui.caption.textContent = `${story} (each line is ten times the one below).`;
  ui.chart.setAttribute("aria-label", `${story}.`);
  drawChart(ui.chart, {
    years,
    series: [people],
    selected: 0,
    t: years.length - 1,
    log: true,
    format: formatPeople,
    label: "Your world",
    cursor: false,
  });
  ui.verdict.textContent = verdict("open", null, done.run.result.state_hash);
  void followVerdict(done.unitId, done.run.result.state_hash, ++shown);
}

/** Counts the worlds shown, so a world's verdict stops updating once another replaces it. */
let shown = 0;

/** Polls what became of the unit until it is settled or another world replaces it. */
async function followVerdict(unitId: string, ours: string, world: number): Promise<void> {
  for (let i = 0; i < 60 && world === shown; i++) {
    try {
      const r = await fetch(`${location.origin}/units/${encodeURIComponent(unitId)}`);
      if (r.ok) {
        const u = (await r.json()) as { status: string; report: { state_hash: string } | null };
        if (world !== shown) return;
        ui.verdict.textContent = verdict(u.status, u.report?.state_hash ?? null, ours);
        if (u.status !== "open") return;
      }
    } catch {
      /* the coordinator may be busy; ask again */
    }
    await new Promise((r) => setTimeout(r, RETRY_MS));
  }
}

/** Waits `ms`, or less if the volunteer stops. */
function pause(ms: number): Promise<void> {
  return new Promise((resolve) => {
    const t = setTimeout(resolve, ms);
    wake = () => {
      clearTimeout(t);
      resolve();
    };
  });
}

/** Set when the volunteer chooses to run on battery too. */
let batteryAllowed = false;

/** True while the device runs on battery, if the browser tells us. */
async function onBattery(): Promise<boolean> {
  if (batteryAllowed) return false;
  const nav = navigator as Navigator & { getBattery?: () => Promise<{ charging: boolean }> };
  try {
    return nav.getBattery ? !(await nav.getBattery()).charging : false;
  } catch {
    return false;
  }
}

let engine: Promise<WebAssembly.Module> | null = null;

async function loop(gen: number): Promise<void> {
  const live = () => running && gen === generation;
  const run: Engine = async (u, p, cb) => {
    const module = await engine!;
    if (!live()) throw new Stopped(); // stopped while its packs downloaded
    return workerEngine(module)(u, p, cb);
  };
  const host = new Host(location.origin, hostName(), run, new BrowserPacks());
  while (live()) {
    try {
      if (await onBattery()) {
        show({ stage: "battery" });
        await pause(RETRY_MS);
        continue;
      }
      if (!engine) {
        show({ stage: "starting" });
        engine = WebAssembly.compileStreaming(fetch("oikos.wasm"));
        engine.catch(() => (engine = null));
      }
      await engine;
      if (!live()) break;
      round = new AbortController();
      const done = await host.round((s) => {
        if (s.stage === "run" && s.years === 0) runStarted = performance.now();
        if (live()) show(s);
      }, round.signal);
      if (!done) {
        show({ stage: "waiting" });
        await pause(RETRY_MS);
        continue;
      }
      tally({ years: done.years });
      showWorld(done);
    } catch (e) {
      if (!live()) break;
      show({ stage: "error", message: e instanceof Error ? e.message : String(e) });
      await pause(RETRY_MS);
    }
  }
}

ui.battery.addEventListener("click", () => {
  batteryAllowed = true;
  wake?.();
});

ui.toggle.addEventListener("click", () => {
  running = !running;
  generation += 1;
  ui.toggle.textContent = running ? "Stop" : "Start computing";
  // While running, the progress is the focal element, not the way to stop it.
  ui.toggle.classList.toggle("btn-tonal", running);
  if (running) {
    void loop(generation);
  } else {
    round?.abort();
    stopRun?.();
    wake?.();
    show({ stage: "idle" });
  }
});

ui.host.textContent = hostName();
tally();
