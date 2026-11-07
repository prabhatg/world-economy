import { describe as group, expect, it } from "vitest";
import { describe, minutesLeft, verdict } from "../src/volunteer/status";

group("volunteer status", () => {
  it("names the simulated year and how far the run has come", () => {
    const line = describe({ stage: "run", years: 4_600, of: 9_000, firstYear: -12_000 }, 4_600 * 30);
    expect(line.headline).toBe("Simulating 7,400 BCE");
    expect(line.detail).toBe("Year 4,600 of 9,000 · about 2 minutes left");
    expect(line.progress).toBeCloseTo(4_600 / 9_000);
  });

  it("estimates the time left only once there is enough pace to go on", () => {
    expect(minutesLeft(100, 9_000, 10_000)).toBeNull();
    expect(minutesLeft(3_000, 9_000, 3 * 60_000)).toBe(6);
    expect(describe({ stage: "run", years: 100, of: 9_000, firstYear: -12_000 }, 5_000).detail).toBe("Year 100 of 9,000");
  });

  it("keeps engine messages off the page", () => {
    expect(describe({ stage: "error", message: "oikos: bad unit\nmore" }).detail).not.toMatch(/oikos/);
  });

  it("says honestly what became of a result", () => {
    expect(verdict("done", "a", "a")).toMatch(/^Accepted/);
    expect(verdict("done", "b", "a")).toMatch(/yours differed/);
    expect(verdict("open", null, "a")).toMatch(/second computer/);
    expect(verdict("failed", null, "a")).toMatch(/could not be finished/);
  });
});
