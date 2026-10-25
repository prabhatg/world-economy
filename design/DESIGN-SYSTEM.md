# ManyWorlds Design System

One visual language for the website, the web viewer (TypeScript), the volunteer apps
(Flutter) and every report and diagram. Tokens live in [`tokens.json`](tokens.json)
(source of truth) and [`tokens.css`](tokens.css) (web). Flutter mapping is in §9.

## 1. Who we design for

Everyone: volunteers with no science background, researchers, policymakers, funders,
students, people on cheap phones and slow connections. Design for the least expert,
most distracted reader first; experts benefit from clarity too.

## 2. Principles (and the research behind them)

| # | Principle | Why it works | In practice |
|---|---|---|---|
| 1 | **One message per region** | Working memory holds about 4 chunks (Cowan 2001); extraneous load competes with understanding (Sweller, cognitive load theory) | Each section has one headline that states the point; at most 4 items per group |
| 2 | **Guide the eye with pre-attentive cues** | Size, contrast, color and motion are processed before conscious attention (Treisman 1980; Healey & Enns 2012) | One focal element per view; the amber accent marks only the single most important thing (Von Restorff isolation effect) |
| 3 | **Group with space and tone, not boxes** | Gestalt proximity and common region group items without the visual noise of borders (Wertheimer; Palmer 1992) | No hard borders or outlines for grouping; use whitespace and soft tonal surfaces |
| 4 | **Headlines carry the meaning** | People scan in F and layer-cake patterns and read the first words of lines (Nielsen Norman Group eye-tracking) | Front-load key words; a reader who reads only headlines gets the story |
| 5 | **Show, then tell** | Words plus pictures beat words alone (Paivio's dual coding; Mayer's multimedia and spatial-contiguity principles) | Every concept gets a diagram; labels sit inside the diagram, next to what they name |
| 6 | **Fewer, bigger choices** | Decision time grows with the number of options (Hick–Hyman law); larger, closer targets are faster (Fitts's law) | One primary action per view; touch targets ≥ 48 px (Material) / 44 pt (Apple) |
| 7 | **Fluency builds trust** | Easy-to-read content is judged more true and more trustworthy (Reber & Schwarz 1999; Alter & Oppenheimer 2009) | Hyper-legible type, high contrast, plain words (≈ grade 8 reading level) |
| 8 | **Show uncertainty honestly** | Point estimates are over-trusted; distributions shown as outcomes are understood better (Padilla et al. 2018; Kale et al. 2019, hypothetical outcome plots) | Fan charts, dot strips of many worlds, ranges in words; never a single confident line for a forecast |
| 9 | **Motion explains causality** | Motion is perceived as cause and effect (Michotte) but distracts when decorative | Animate only to show change or connection; ≤ 400 ms; honor reduced-motion |
| 10 | **Calm by default** | Calm technology lives in the periphery until needed (Weiser & Brown 1996) | Volunteer apps stay quiet; status at a glance, details on request |
| 11 | **Respect autonomy** | Persuasion must not manipulate (self-determination theory, Deci & Ryan); dark patterns erode trust | No dark patterns, fake urgency or guilt; credit people visibly and fairly |
| 12 | **Accessible is the baseline** | ~1 in 12 men have color-vision deficiency; many users have low vision or motor limits | WCAG 2.2 AA (4.5:1 text, 3:1 large text and UI); never encode meaning by color alone |

## 3. Color

Tonal, calm neutrals with a slight teal bias; one warm accent.

| Role | Purpose |
|---|---|
| `primary` (amber, "the better path") | The single most important action or data series in a view. Use sparingly. |
| `secondary` (deep teal) | Links, secondary emphasis, the "history" series |
| `surface`, `surface-container-low/high` | Background and tonal grouping layers (Material 3 tonal elevation) instead of borders |
| `on-surface`, `on-surface-variant` | Primary and secondary text |
| `night`, `on-night` | The always-dark "deep time" band used for heroes and the viewer's globe |
| `outline-variant` | Only for data-viz gridlines and focus rings, never for boxing content |

**Welfare dimensions** use a color-blind-safe set (Okabe & Ito 2008), always paired
with an icon and a label:

| Dimension | Token | Color |
|---|---|---|
| Health & longevity | `welfare-health` | vermillion |
| Material wellbeing | `welfare-material` | orange |
| Capabilities & freedom | `welfare-freedom` | blue |
| Sustainability | `welfare-sustain` | bluish green |

## 4. Typography

| Role | Face | Why |
|---|---|---|
| Display | **Bricolage Grotesque** (variable, optical sizes) | Warm, contemporary, distinctive at large sizes; used with restraint |
| Body and UI | **Atkinson Hyperlegible Next** | Designed by the Braille Institute for maximum character distinction; supports principle 7 |
| Data and code | **Atkinson Hyperlegible Mono** | Same legibility goals, tabular digits for numbers |

Headlines may pair a light and a bold weight in one phrase ("Run a world. **Help find a
better one.**"): the bold half carries the message. Body text 17 px (Apple body size),
line length 60–70 characters, line height 1.55. Uppercase labels get +0.06em tracking.
Type scale (fluid): display 48–84 px, headline 30–44 px, title 20–24 px, body 17 px,
label 13 px.

## 5. Space, shape, elevation

- **4 pt grid**: 4, 8, 12, 16, 24, 32, 48, 64, 96, 128.
- **Section rhythm**: 96–128 px between sections on desktop, 64 px on phones.
- **Radii**: 8 (chips), 16 (media), 28 (large surfaces, Material 3 "large"), full (pills).
- **Elevation**: tonal layers first; one soft shadow level only for floating elements
  (menus, the sticky nav on scroll). No hard drop shadows, no borders around cards.

## 6. Diagrams and data visualization

Diagrams are the primary medium, not decoration.

1. **One idea per diagram**, stated in its caption as a full sentence.
2. **Label directly** on the marks; avoid legends where possible.
3. **History is teal, the better path is amber, other worlds are muted.** Keep this
   mapping everywhere so readers learn it once.
4. **Show ensembles, not single runs**: fan charts, faint spaghetti lines, dot strips.
5. Thin strokes (1.5 px), rounded caps, generous whitespace, no 3D, no chartjunk
   (Tufte's data–ink ratio).
6. Real units and scales only (years BCE/CE, people, H3 cells); illustrative diagrams
   say "illustrative" in the caption.
7. Diagrams use theme tokens, so they read in light and dark.

## 7. Components (core set)

- **Button**: pill shape; *filled amber* for the one primary action, *tonal* for
  secondary, *text* for tertiary. Labels are verbs ("Read the design", "Join the alpha").
- **Surface**: soft tonal background (`surface-container-low`), radius 28, no border.
- **Step**: number + title + one sentence + mini diagram; only for real sequences.
- **Stat**: big number + unit + short label; only when the number is the point.
- **Chip**: small tonal label for status ("In progress", "Planned").
- **Nav**: minimal, sticky, translucent with backdrop blur; ≤ 5 links.

## 8. Voice and tone

Plain, warm, specific, honest. Short sentences. Say what we know and what we don't.
Never overclaim ("will solve poverty"); prefer "help find", "test", "measure".
Name things the way people know them ("your computer", not "compute node").

## 9. Platform mapping

| Token | Material 3 / Flutter `ColorScheme` | Apple HIG |
|---|---|---|
| `primary` / `on-primary` | `primary` / `onPrimary` | tint color |
| `secondary` / `on-secondary` | `secondary` / `onSecondary` | secondary tint |
| `surface` | `surface` | systemBackground |
| `surface-container-low` | `surfaceContainerLow` | secondarySystemBackground |
| `surface-container-high` | `surfaceContainerHigh` | tertiarySystemBackground |
| `on-surface` / `on-surface-variant` | `onSurface` / `onSurfaceVariant` | label / secondaryLabel |
| `outline-variant` | `outlineVariant` | separator |

- Flutter: build `ThemeData(useMaterial3: true, colorScheme: ...)` from `tokens.json`
  (generate a Dart file from the JSON so nothing is copied by hand); Dynamic Type via
  `MediaQuery.textScaler`; respect `MediaQuery.disableAnimations`.
- Apple platforms: support Dynamic Type sizes and Reduce Motion; SF Symbols-like
  line icons at 1.5 px stroke.
- Web: `tokens.css` custom properties with `prefers-color-scheme` and `data-theme`.

## 10. Review

Every user-facing change gets a review by the design reviewer agent
(`.claude/agents/design-reviewer.md`) against this document before it ships.
