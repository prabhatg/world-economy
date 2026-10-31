import { describe, expect, it } from "vitest";
import { formatPeople, formatShare, formatYear } from "../src/format";

describe("format", () => {
  it("writes years BCE and CE with grouping", () => {
    expect(formatYear(-12000)).toBe("12,000 BCE");
    expect(formatYear(-3000)).toBe("3,000 BCE");
    expect(formatYear(1500)).toBe("1500 CE");
  });

  it("rounds people the way people say them", () => {
    expect(formatPeople(412)).toBe("412");
    expect(formatPeople(9_999)).toBe("9,999");
    expect(formatPeople(550_520)).toBe("551,000");
    expect(formatPeople(2_321_520)).toBe("2.3 million");
    expect(formatPeople(1_000_000)).toBe("1 million");
    expect(formatPeople(12_443_637)).toBe("12 million");
  });

  it("writes shares as whole percentages, keeping small ones visible", () => {
    expect(formatShare(0.767)).toBe("77%");
    expect(formatShare(0)).toBe("0%");
    expect(formatShare(0.003)).toBe("under 1%");
    expect(formatShare(1)).toBe("100%");
  });
});
