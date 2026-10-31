/**
 * ManyWorlds viewer v0 (ADR-0063): plays back an ensemble of plausible worlds. The map
 * shows one world's field at one time, read lazily from that world's fields.zarr; the
 * charts show every world, so a reader always sees one world against the many that
 * fit the record.
 */

import "../../design/tokens.css";
import "./styles.css";
import { type Catalog, loadCatalog } from "./catalog";
import { type CellPolygon, cellPolygons } from "./cells";
import { drawChart } from "./chart";
import { FIELDS, type Field, binOf, legendLabels, valueOf } from "./fields";
import { formatPeople, formatShare, formatYear } from "./format";
import { FieldMap, type Place, type Rgba, tokenColor } from "./map";
import { ZarrArray } from "./zarr";

/** Milliseconds per recorded time while playing (a century in M1). */
const STEP_MS = 300;

/** Region names for M1 (Southwest Asia), placed by hand. */
const PLACES: Place[] = [
  { name: "Anatolia", lng: 33.0, lat: 39.2 },
  { name: "Levant", lng: 36.6, lat: 33.0 },
  { name: "Mesopotamia", lng: 44.0, lat: 33.4 },
  { name: "Zagros", lng: 47.6, lat: 35.0 },
  { name: "Nile", lng: 31.2, lat: 27.0 },
  { name: "Cyprus", lng: 33.2, lat: 34.6 },
];

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const ui = {
  title: $("title"),
  headline: $("headline"),
  summary: $("summary"),
  range: $("range"),
  firstYear: $("first-year"),
  lastYear: $("last-year"),
  peopleCaption: $("people-caption"),
  mapSummary: $("map-summary"),
  play: $<HTMLButtonElement>("play"),
  slider: $<HTMLInputElement>("slider"),
  fields: $<HTMLFieldSetElement>("fields"),
  world: $<HTMLSelectElement>("world"),
  year: $("year"),
  mapWorld: $("map-world"),
  status: $("status"),
  legend: $("legend"),
  people: document.getElementById("chart-people") as unknown as SVGSVGElement,
  farming: document.getElementById("chart-farming") as unknown as SVGSVGElement,
  note: $("note"),
  provenance: $("provenance"),
  map: $<HTMLDivElement>("map"),
};

const base = new URLSearchParams(location.search).get("data") ?? "data/m1/";
const arrays = new Map<string, Promise<ZarrArray>>();
const open = (path: string) => {
  let a = arrays.get(path);
  if (!a) {
    a = ZarrArray.open(base + path);
    a.catch(() => arrays.delete(path));
    arrays.set(path, a);
  }
  return a;
};

let catalog: Catalog;
let fields: Field[];
let map: FieldMap;
let cells: CellPolygon[];
let ramp: Rgba[];
let land: Rgba;
const state = { world: 0, field: 0, t: 0, playing: false };
let shown: { values: Float64Array; dry: Float64Array | null } | null = null;
let drawing = 0;
let timer = 0;

function setStatus(text: string) {
  ui.status.textContent = text;
}

async function start() {
  catalog = await loadCatalog(`${base}catalog.json`);
  const recorded = new Set(catalog.fields.map((f) => f.name));
  fields = FIELDS.filter((f) => recorded.has(f.name));
  const [first, last] = [formatYear(catalog.years[0]), formatYear(catalog.years.at(-1)!)];
  const n = catalog.worlds.length;
  document.title = `${catalog.title} · ManyWorlds viewer`;
  ui.title.textContent = "ManyWorlds viewer";
  ui.headline.textContent = `How people spread and took up farming in Southwest Asia: ${n} possible histories`;
  ui.summary.textContent = `${n} simulated worlds that match the evidence checked so far: how many dated sites \
and villages there are, where obsidian came from, and when barley changed into a crop. Known gap: in every world \
people live mostly on planted crops thousands of years too early, because nothing checked yet dates when planting began.`;
  ui.firstYear.textContent = first;
  ui.lastYear.textContent = last;
  const end = catalog.worlds.map((w) => w.series.people.at(-1)!);
  ui.range.textContent = `The worlds that fit hold ${formatPeople(Math.min(...end))} to ${formatPeople(
    Math.max(...end),
  )} people by ${last}`;
  ui.world.replaceChildren(...catalog.worlds.map((_, i) => new Option(worldName(i), String(i))));
  ui.fields.append(
    ...fields.map((f, i) => {
      const label = document.createElement("label");
      label.className = "field-option";
      label.innerHTML = `<input type="radio" name="field" value="${i}" ${i === 0 ? "checked" : ""}><span>${f.label}</span>`;
      return label;
    }),
  );
  ui.slider.max = String(catalog.years.length - 1);
  renderProvenance();

  ramp = [1, 2, 3, 4, 5, 6].map((k) => tokenColor(`--map-ramp-${k}`));
  land = tokenColor("--map-land");
  setStatus("Loading the map…");
  const h3 = await (await open(`${catalog.worlds[0].store}/cell_h3`)).bigints();
  cells = cellPolygons(h3);
  map = new FieldMap(ui.map, cells, describe, PLACES);

  ui.play.addEventListener("click", () => (state.playing ? pause() : play()));
  ui.slider.addEventListener("input", () => {
    pause();
    state.t = Number(ui.slider.value);
    void draw();
  });
  ui.fields.addEventListener("change", (e) => {
    state.field = Number((e.target as HTMLInputElement).value);
    void draw();
  });
  ui.world.addEventListener("change", () => selectWorld(Number(ui.world.value)));
  for (const el of [ui.play, ui.slider, ui.world]) el.disabled = false;
  await draw();
}

/** A world named by what it does, not by its calibration run (that is in provenance). */
function worldName(i: number): string {
  const people = formatPeople(catalog.worlds[i].series.people.at(-1)!);
  const defaults = catalog.worlds[i].label === "Calibrated defaults" ? ", defaults" : "";
  return `World ${i + 1}: ${people} people${defaults}`;
}

function selectWorld(i: number) {
  state.world = i;
  ui.world.value = String(i);
  void draw();
}

function play() {
  if (state.t >= catalog.years.length - 1) state.t = 0;
  state.playing = true;
  ui.play.textContent = "Pause";
  void tick();
}

function pause() {
  state.playing = false;
  clearTimeout(timer);
  ui.play.textContent = state.t >= catalog.years.length - 1 ? "Play again" : "Play";
  announce();
}

async function tick() {
  const started = performance.now();
  await draw();
  if (!state.playing) return;
  if (state.t >= catalog.years.length - 1) return pause();
  timer = window.setTimeout(() => {
    state.t++;
    void tick();
  }, Math.max(0, STEP_MS - (performance.now() - started)));
}

/** The values of the current field and the dry land at the current time. */
async function read(world: number, field: Field, t: number) {
  const store = catalog.worlds[world].store;
  const hasLand = catalog.fields.some((f) => f.name === "land") || (await hasArray(store, "land"));
  const [values, dry] = await Promise.all([
    open(`${store}/${field.name}`).then((a) => a.row(t)),
    hasLand ? open(`${store}/land`).then((a) => a.row(t)) : Promise.resolve(null),
  ]);
  return { values, dry };
}

const landKnown = new Map<string, Promise<boolean>>();
function hasArray(store: string, name: string): Promise<boolean> {
  const key = `${store}/${name}`;
  if (!landKnown.has(key)) landKnown.set(key, open(key).then(() => true, () => false));
  return landKnown.get(key)!;
}

async function draw() {
  const id = ++drawing;
  const { world, t } = state;
  const field = fields[state.field];
  let data;
  try {
    data = await read(world, field, t);
  } catch (e) {
    setStatus(`Could not read this world: ${(e as Error).message}`);
    return;
  }
  if (id !== drawing) return; // a newer draw started while this one loaded
  setStatus("");
  shown = data;
  const rgba = new Uint8ClampedArray(cells.length * 4);
  for (let i = 0; i < cells.length; i++) {
    if (data.dry && data.dry[i] === 0) continue; // sea, or under a lake: not drawn
    const bin = binOf(field, valueOf(field, data.values[i]));
    rgba.set(bin < 0 ? land : ramp[bin], i * 4);
  }
  map.paint(rgba);
  renderTime();
  renderLegend(field);
  renderCharts();
  if (!state.playing) announce();
  if (state.playing && t + 1 < catalog.years.length) void read(world, field, t + 1).catch(() => {});
}

function describe(i: number): string | null {
  if (!shown || (shown.dry && shown.dry[i] === 0)) return null;
  const field = fields[state.field];
  const v = valueOf(field, shown.values[i]);
  return `${field.label}: ${field.name === "population" ? `${formatPeople(v)} people` : field.format(v)}`;
}

/** One sentence describing the map, for screen readers, whenever it settles. */
function announce() {
  if (!shown) return;
  const field = fields[state.field];
  const labels = legendLabels(field);
  let landCells = 0, some = 0, high = 0;
  for (let i = 0; i < cells.length; i++) {
    if (shown.dry && shown.dry[i] === 0) continue;
    landCells++;
    const bin = binOf(field, valueOf(field, shown.values[i]));
    if (bin >= 0) some++;
    if (bin >= 3) high++;
  }
  const pct = (k: number) => formatShare(landCells ? k / landCells : 0);
  ui.mapSummary.textContent = `${formatYear(catalog.years[state.t])}, world ${state.world + 1}. ${field.legend}: \
${pct(some)} of the land is at ${labels[0]} or more, ${pct(high)} at ${labels[3]} or more.`;
}

function renderTime() {
  const year = formatYear(catalog.years[state.t]);
  ui.year.textContent = year;
  ui.slider.value = String(state.t);
  ui.slider.setAttribute("aria-valuetext", year);
  ui.mapWorld.textContent = `World ${state.world + 1} of ${catalog.worlds.length}`;
}

function renderLegend(field: Field) {
  const swatch = (c: Rgba) => `<span class="legend-swatch" style="background: rgb(${c[0]} ${c[1]} ${c[2]})"></span>`;
  const label = (text: string) => `<span class="legend-label">${text}</span>`;
  const below = field.name === "population" ? "0" : "none";
  ui.legend.innerHTML = `<p class="legend-title">${field.legend}</p>
    <div class="legend-scale" aria-hidden="true">${swatch(land)}${ramp.map(swatch).join("")}
    ${label(below)}${legendLabels(field).map(label).join("")}</div>`;
  ui.legend.setAttribute("role", "img");
  ui.legend.setAttribute(
    "aria-label",
    `${field.legend}: six classes from ${field.format(field.thresholds[0])} to over ${field.format(field.thresholds[5])}, lighter is more`,
  );
}

function renderCharts() {
  const { years, worlds } = catalog;
  const people = worlds.map((w) => w.series.people);
  const farming = worlds.map((w) => w.series.farming);
  const common = { years, selected: state.world, t: state.t, onSelect: selectWorld };
  drawChart(ui.people, { ...common, series: people, log: true, format: formatPeople });
  drawChart(ui.farming, { ...common, series: farming, log: false, format: formatShare });
  const now = people.map((s) => s[state.t]);
  const [lo, hi] = [Math.min(...now), Math.max(...now)];
  const year = formatYear(years[state.t]);
  ui.people.setAttribute(
    "aria-label",
    `People in ${worlds.length} worlds at ${year}: from ${formatPeople(lo)} to ${formatPeople(hi)}; the world on the map has ${formatPeople(now[state.world])}.`,
  );
  ui.farming.setAttribute(
    "aria-label",
    `Share of food grown at ${year}: the world on the map ${formatShare(farming[state.world][state.t])}.`,
  );
  ui.peopleCaption.textContent = `People in every world, on a log scale. The bold line is the world on the map; \
the first century is the model settling from its start.`;
  ui.note.textContent = `The evidence checked so far does not tell these worlds apart, so read any one of them \
as one possibility, not what happened. In all of them farming spreads far too early: none of the records checked \
yet dates when people began to plant.`;
}

function renderProvenance() {
  const p = catalog.provenance;
  const row = (k: string, v: string) => `<dt>${k}</dt><dd>${v}</dd>`;
  const worlds = catalog.worlds
    .map((w, i) => row(`World ${i + 1}`, `${w.label}, state ${w.state_hash.slice(0, 12)}`))
    .join("");
  ui.provenance.innerHTML = `<p>Each world is a run of the Oikos engine from the same start, with parameters that \
the calibration found consistent with four records: radiocarbon dates (p3k14c), barley rachis remains (Allaby et \
al. 2017), survey sites (Palmisano et al. 2021) and sourced obsidian (DObsiSS), each with one random seed. Forcing \
packs are the climate, coastlines, rivers and obsidian sources every world shares, built from published datasets.</p>
    <dl>${row("Engine", p.engine)}${row("Calibration", p.best)}${row("Seed", String(p.seed))}${row(
      "Forcing packs",
      p.packs.map((k) => `${k.kind} ${k.id.slice(0, 8)}`).join(", "),
    )}${worlds}</dl>`;
}

start().catch((e) => {
  ui.headline.textContent = "No worlds to show";
  setStatus((e as Error).message);
});
