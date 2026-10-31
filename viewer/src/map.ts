/**
 * The map: one world's field on the always-dark night surface (design system §3).
 * Cells are drawn from their H3 boundaries; sea and cells without dry land in that
 * century are left out, so coastlines appear without a basemap.
 */

import { Deck, MapView, WebMercatorViewport } from "@deck.gl/core";
import { SolidPolygonLayer, TextLayer } from "@deck.gl/layers";
import { type CellPolygon, bounds } from "./cells";

export type Rgba = [number, number, number, number];

/** A few places, so readers can find their way without a basemap. */
export interface Place {
  name: string;
  lng: number;
  lat: number;
}

/** A token color (#RRGGBB) from the page's CSS custom properties. */
export function tokenColor(name: string, el: Element = document.documentElement): Rgba {
  const hex = getComputedStyle(el).getPropertyValue(name).trim();
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) throw new Error(`design token ${name} is "${hex}", expected #RRGGBB (is design/tokens.css loaded?)`);
  const v = parseInt(m[1], 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255, 255];
}

export class FieldMap {
  private readonly deck: Deck<MapView>;
  private colors: Uint8ClampedArray = new Uint8ClampedArray(0);
  private version = 0;

  constructor(
    private readonly parent: HTMLDivElement,
    private readonly cells: CellPolygon[],
    describe: (index: number) => string | null,
    private readonly places: Place[] = [],
  ) {
    const { width, height } = parent.getBoundingClientRect();
    const fit = new WebMercatorViewport({ width: Math.max(width, 320), height: Math.max(height, 240) }).fitBounds(
      bounds(cells),
      // Room at the top for the year and at the bottom for the legend.
      { padding: { top: 88, bottom: 16, left: 16, right: 16 } },
    );
    this.deck = new Deck<MapView>({
      parent,
      views: new MapView({ repeat: false }),
      initialViewState: { longitude: fit.longitude, latitude: fit.latitude, zoom: fit.zoom, minZoom: 3, maxZoom: 9 },
      controller: { dragRotate: false, touchRotate: false },
      // Vertical swipes scroll the page on phones; horizontal drags and pinches move the map.
      touchAction: "pan-y",
      layers: [],
      onLoad: () => {
        const canvas = parent.querySelector("canvas");
        canvas?.setAttribute("aria-label", "Map: arrow keys pan, plus and minus zoom");
        canvas?.setAttribute("role", "img");
      },
      getTooltip: ({ object }) => {
        const text = object ? describe((object as CellPolygon).index) : null;
        return text ? { text, className: "map-tooltip" } : null;
      },
    });
  }

  /** Colors every cell (rgba per cell, in cell order); alpha 0 hides a cell. */
  paint(rgba: Uint8ClampedArray): void {
    this.colors = rgba;
    this.version++;
    const c = this.colors;
    this.deck.setProps({
      layers: [
        new SolidPolygonLayer<CellPolygon>({
          id: "cells",
          data: this.cells,
          getPolygon: (d) => d.polygon,
          getFillColor: (d) => [c[d.index * 4], c[d.index * 4 + 1], c[d.index * 4 + 2], c[d.index * 4 + 3]],
          updateTriggers: { getFillColor: this.version },
          pickable: true,
        }),
        new TextLayer<Place>({
          id: "places",
          data: this.places,
          getPosition: (d) => [d.lng, d.lat],
          getText: (d) => d.name,
          getColor: tokenColor("--on-night"),
          getSize: 14,
          fontFamily: getComputedStyle(document.documentElement).getPropertyValue("--font-body"),
          fontWeight: 700,
          outlineWidth: 4,
          outlineColor: tokenColor("--night"),
          fontSettings: { sdf: true },
        }),
      ],
    });
  }

  get element(): HTMLDivElement {
    return this.parent;
  }
}
