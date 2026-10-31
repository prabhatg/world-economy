import { describe, expect, it } from "vitest";
import { parseCatalog } from "../src/catalog";

const valid = {
  format: 1,
  title: "Neolithic Southwest Asia",
  summary: "Plausible worlds",
  h3_resolution: 5,
  years: [-12000, -11900],
  fields: [{ name: "population", label: "People", unit: "people", description: "People" }],
  worlds: [
    { id: "w0", label: "Calibrated defaults", params: {}, store: "w0/fields.zarr", state_hash: "ab",
      series: { people: [1, 2], farming: [0, 0.1], domestic: [0, 0.5] } },
  ],
  provenance: { engine: "oikos 0.1.0", best: "runs/m1/round5/best.json", seed: 1, packs: [] },
};

describe("catalog", () => {
  it("accepts what `python -m oikos_cal ensemble` writes", () => {
    expect(parseCatalog(valid).worlds[0].series.people).toEqual([1, 2]);
  });

  it("rejects other formats and series of the wrong length, saying what to fix", () => {
    expect(() => parseCatalog({ ...valid, format: 2 })).toThrow(/format 2/);
    const short = { ...valid, worlds: [{ ...valid.worlds[0], series: { ...valid.worlds[0].series, people: [1] } }] };
    expect(() => parseCatalog(short)).toThrow(/w0: series people has 1 values, years 2/);
    expect(() => parseCatalog({ ...valid, worlds: [] })).toThrow(/no worlds/);
  });
});
