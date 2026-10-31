/** H3 cells as polygons for the map; geometry comes from h3-js, never from the run. */

import { cellToBoundary } from "h3-js";

export interface CellPolygon {
  /** Position in the world's cell order (fields.zarr `cell_h3`). */
  index: number;
  /** Closed ring of [lng, lat]. */
  polygon: [number, number][];
}

export function h3String(index: bigint): string {
  return index.toString(16);
}

export function cellPolygons(h3: BigUint64Array | bigint[]): CellPolygon[] {
  return Array.from(h3, (c, index) => ({ index, polygon: cellToBoundary(h3String(c), true) as [number, number][] }));
}

/** [[west, south], [east, north]] of the cells. */
export function bounds(cells: CellPolygon[]): [[number, number], [number, number]] {
  let [w, s, e, n] = [180, 90, -180, -90];
  for (const { polygon } of cells) {
    for (const [lng, lat] of polygon) {
      w = Math.min(w, lng);
      e = Math.max(e, lng);
      s = Math.min(s, lat);
      n = Math.max(n, lat);
    }
  }
  return [[w, s], [e, n]];
}
