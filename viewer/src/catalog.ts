/**
 * The ensemble a viewer shows, as `python -m oikos_cal ensemble` writes it
 * (calibration/oikos_cal/ensemble.py): plausible worlds, each with its own
 * fields.zarr, and the series the charts draw.
 */

export interface World {
  id: string;
  label: string;
  params: Record<string, number>;
  store: string;
  state_hash: string;
  series: { people: number[]; farming: number[]; domestic: number[] };
}

export interface Catalog {
  format: 1;
  title: string;
  summary: string;
  h3_resolution: number;
  years: number[];
  fields: { name: string; label: string; unit: string; description: string }[];
  worlds: World[];
  provenance: { engine: string; best: string; seed: number; packs: { kind: string; id: string }[] };
}

export const BUILD_HINT =
  "Build one with: (cd calibration && PYTHONPATH=.:../data python -m oikos_cal ensemble " +
  "runs/m1/round6/best.json --out ../viewer/data/m1)";

export function parseCatalog(json: any): Catalog {
  if (json?.format !== 1) throw new Error(`catalog format ${json?.format}, this viewer reads format 1`);
  if (!Array.isArray(json.worlds) || json.worlds.length === 0) throw new Error(`the catalog has no worlds. ${BUILD_HINT}`);
  const n = json.years.length;
  for (const w of json.worlds) {
    for (const [name, values] of Object.entries(w.series as Record<string, number[]>)) {
      if (values.length !== n) throw new Error(`${w.id}: series ${name} has ${values.length} values, years ${n}`);
    }
  }
  return json as Catalog;
}

export async function loadCatalog(url: string): Promise<Catalog> {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`No ensemble at ${url} (HTTP ${r.status}). ${BUILD_HINT}`);
  return parseCatalog(await r.json());
}
