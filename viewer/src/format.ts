/** Plain-language numbers for labels (design system §8: name things as people know them). */

const grouped = new Intl.NumberFormat("en-US");

export function formatYear(year: number): string {
  return year < 0 ? `${grouped.format(-year)} BCE` : `${year} CE`;
}

export function formatPeople(n: number): string {
  if (n >= 10_000_000) return `${Math.round(n / 1e6)} million`;
  if (n >= 1_000_000) return `${(n / 1e6).toFixed(1).replace(/\.0$/, "")} million`;
  if (n >= 10_000) return grouped.format(Math.round(n / 1000) * 1000);
  return grouped.format(Math.round(n));
}

export function formatShare(x: number): string {
  if (x > 0 && x < 0.005) return "under 1%";
  return `${Math.round(x * 100)}%`;
}
