import { describe, expect, it } from "vitest";
import { FIELDS, binOf, legendLabels, valueOf } from "../src/fields";

describe("fields", () => {
  it("has at most four choices, each with six classes for the six ramp steps", () => {
    expect(FIELDS.length).toBeLessThanOrEqual(4);
    for (const f of FIELDS) {
      expect(f.thresholds).toHaveLength(6);
      expect([...f.thresholds].sort((a, b) => a - b)).toEqual(f.thresholds);
    }
  });

  it("spreads people over orders of magnitude, as settlement does", () => {
    const people = FIELDS.find((f) => f.name === "population")!;
    const t = people.thresholds;
    expect(t[5] / t[0]).toBeGreaterThanOrEqual(500);
  });

  it("puts values in classes: below the first threshold is empty land", () => {
    const people = FIELDS.find((f) => f.name === "population")!;
    expect(binOf(people, 0)).toBe(-1);
    expect(binOf(people, people.thresholds[0])).toBe(0);
    expect(binOf(people, people.thresholds[5] * 10)).toBe(5);
  });

  it("reads shares from parts per 10,000 and shows soil as soil lost", () => {
    const soil = FIELDS.find((f) => f.name === "soil")!;
    expect(valueOf(soil, 10_000)).toBe(0);
    expect(valueOf(soil, 7_500)).toBeCloseTo(0.25);
    const farming = FIELDS.find((f) => f.name === "cultivation")!;
    expect(valueOf(farming, 5_000)).toBeCloseTo(0.5);
    expect(valueOf(FIELDS.find((f) => f.name === "population")!, 412)).toBe(412);
  });
});

describe("legend labels", () => {
  it("show each class's lower bound, small shares exactly, and the top class as open", () => {
    const label = (name: string) => FIELDS.find((f) => f.name === name)!;
    expect(legendLabels(label("population"))).toEqual(["1", "5", "20", "60", "200", "800+"]);
    expect(legendLabels(label("obsidian_cappadocia"))).toEqual(["0.1%", "1%", "3%", "10%", "30%", "90%+"]);
    expect(legendLabels(label("cultivation"))[5]).toBe("95%+");
  });

  it("say what is measured in plain words", () => {
    expect(FIELDS.find((f) => f.name === "obsidian_cappadocia")!.legend).toMatch(/reaching people \(100% at the volcanoes\)/);
    expect(FIELDS.find((f) => f.name === "cultivation")!.legend).toMatch(/wild or domestic/);
  });
});
