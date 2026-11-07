// What the volunteer page says, from what the host is doing: one headline and one
// detail line (design system §8: plain words, say what we know and what we don't).

import { formatYear } from "../format";
import type { Stage } from "./host";

const grouped = new Intl.NumberFormat("en-US");

export type State =
  | { stage: "idle" }
  | { stage: "starting" }
  | Stage
  | { stage: "waiting" }
  | { stage: "battery" }
  | { stage: "error"; message: string };

export interface Line {
  headline: string;
  detail: string;
  /** Share done, 0–1, or null when there is nothing to measure. */
  progress: number | null;
}

/** Minutes left at the pace so far, once there is enough of it to go on. */
export function minutesLeft(years: number, of: number, elapsedMs: number): number | null {
  if (years < 200 || elapsedMs <= 0) return null;
  return Math.max(1, Math.round(((of - years) * elapsedMs) / years / 60_000));
}

export function describe(s: State, elapsedMs = 0): Line {
  switch (s.stage) {
    case "idle":
      return { headline: "Ready when you are", detail: "Nothing runs until you press Start.", progress: null };
    case "starting":
      return { headline: "Getting the engine", detail: "Downloaded once, then kept by your browser.", progress: null };
    case "packs":
      return {
        headline: "Getting the world's land and climate",
        detail: `File ${s.done + 1} of ${s.of}. Downloaded once, then kept in this browser.`,
        progress: null,
      };
    case "run": {
      const left = minutesLeft(s.years, s.of, elapsedMs);
      const pace = left === null ? "" : ` · about ${left} minute${left === 1 ? "" : "s"} left`;
      return {
        headline: `Simulating ${formatYear(s.firstYear + s.years)}`,
        detail: `Year ${grouped.format(s.years)} of ${grouped.format(s.of)}${pace}`,
        progress: s.of > 0 ? s.years / s.of : null,
      };
    }
    case "upload":
      return { headline: "Sending your result", detail: "The record of your world, to be checked and used.", progress: null };
    case "waiting":
      return { headline: "No worlds to compute right now", detail: "Checking again in a minute.", progress: null };
    case "battery":
      return { headline: "Paused while on battery", detail: "Plug in and it carries on by itself.", progress: null };
    case "error": // the reason goes to the console: engine messages are for developers
      return { headline: "That world did not finish", detail: "Something went wrong. Trying again in a minute.", progress: null };
  }
}

/** What became of a result this browser sent, from the coordinator's view of its unit. */
export function verdict(status: string, acceptedHash: string | null, ours: string): string {
  if (status === "done") {
    return acceptedHash === ours
      ? "Accepted: your result was checked and kept."
      : "Another result was accepted; yours differed and was set aside.";
  }
  if (status === "failed") return "This world could not be finished by any computer.";
  return "Waiting for a second computer to check it.";
}
