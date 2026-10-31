import { describe, expect, it } from "vitest";
import { cellPolygons, h3String } from "../src/cells";

describe("cells", () => {
  it("turns the engine's uint64 H3 indexes into h3-js strings", () => {
    expect(h3String(0x85283473fffffffn)).toBe("85283473fffffff");
  });

  it("gives each cell a closed ring of [lng, lat] vertices", () => {
    const [p] = cellPolygons([0x85283473fffffffn]);
    expect(p.index).toBe(0);
    expect(p.polygon.length).toBe(7);
    expect(p.polygon[0]).toEqual(p.polygon[6]);
    const [lng, lat] = p.polygon[0];
    expect(lat).toBeGreaterThan(37);
    expect(lat).toBeLessThan(38);
    expect(lng).toBeGreaterThan(-123);
    expect(lng).toBeLessThan(-121);
  });
});
