/**
 * What the map can show: at most four choices (design system: about four items per
 * group), each in six classes for the six steps of the map ramp. Classes are fixed
 * across worlds and times, so colors compare between them; they step about evenly in
 * the logarithm where values span orders of magnitude (people per cell run from 1 to
 * about 2,000 in M1 worlds; obsidian falls off exponentially). Shares are recorded in
 * parts per 10,000 (engine `FIELD_ONE`).
 */

import { formatPeople, formatShare } from "./format";

export interface Field {
  /** Array name in fields.zarr. */
  name: string;
  label: string;
  /** Legend title: what one cell's color means. */
  legend: string;
  /** Lower bounds of the six classes; values below the first are empty land. */
  thresholds: number[];
  /** Recorded value to the value shown. */
  read: (raw: number) => number;
  format: (v: number) => string;
}

const share = (raw: number) => raw / 10_000;

export const FIELDS: Field[] = [
  {
    name: "population",
    label: "People",
    legend: "People per cell (about 250 km²)",
    thresholds: [1, 5, 20, 60, 200, 800],
    read: (raw) => raw,
    format: formatPeople,
  },
  {
    name: "cultivation",
    label: "Farming",
    legend: "Share of food from planted crops, wild or domestic",
    thresholds: [0.01, 0.2, 0.4, 0.6, 0.8, 0.95],
    read: share,
    format: formatShare,
  },
  {
    name: "soil",
    label: "Soil lost",
    legend: "Share of the soil worn away by farming",
    thresholds: [0.01, 0.05, 0.1, 0.2, 0.4, 0.6],
    read: (raw) => 1 - share(raw),
    format: formatShare,
  },
  {
    name: "obsidian_cappadocia",
    label: "Obsidian",
    legend: "Cappadocian obsidian reaching people (100% at the volcanoes)",
    thresholds: [0.001, 0.01, 0.03, 0.1, 0.3, 0.9],
    read: share,
    format: formatShare,
  },
];

/** Lower bounds of the classes, the top class open ("800+"); small shares to 0.1%. */
export function legendLabels(field: Field): string[] {
  return field.thresholds.map((t, i) => {
    const text = field.format === formatShare && t < 0.01 ? `${(t * 100).toFixed(1)}%` : field.format(t);
    return i === field.thresholds.length - 1 ? `${text}+` : text;
  });
}

export function valueOf(field: Field, raw: number): number {
  return field.read(raw);
}

/** Class 0-5 of a shown value, or -1 below the first threshold. */
export function binOf(field: Field, value: number): number {
  let bin = -1;
  for (const t of field.thresholds) if (value >= t) bin++;
  return bin;
}
