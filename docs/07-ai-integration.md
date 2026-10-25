# 7. AI integration

AI is used where it multiplies human-scale effort — reading the literature, building
behavioral models, and making expensive simulations cheap enough to calibrate and
search — **not** as a per-agent runtime brain for billions of agents.

## 7.1 Knowledge ingestion ("all of humanity's knowledge, with citations")

Pipeline (LLM agents + human review):
1. Corpus: papers, books, datasets, meta-analyses per domain.
2. Extraction: candidate mechanisms (causal claims with conditions), parameter
   estimates (value, uncertainty, units, context, population), stylized facts.
3. Normalization: map to the parameter registry and mechanism catalog; convert units.
4. Aggregation: meta-analytic priors (e.g. random-effects pooling) per parameter.
5. Provenance: every prior links to source passages; confidence scores; reviewer sign-off.
6. Contradiction detection: competing claims → competing mechanism modules.

Output: a versioned **knowledge base** (parameters, priors, mechanisms, stylized facts)
that the engine and calibration consume.

## 7.2 Agent archetypes & behavior

- LLMs help design **archetypes** (forager band, Neolithic farmer household, Bronze
  Age elite, medieval peasant, merchant, industrial worker, modern smallholder,
  urban informal worker, bureaucrat, politician…) and their decision rules, grounded
  in ethnographic and behavioral-economics evidence.
- Runtime behavior = compact policy functions shared across all agents of an archetype
  (parametric or small neural nets, batched on GPU). Optionally distilled from LLM
  judgments on sampled situations, then calibrated against data (cf. AgentTorch's
  "LLM archetypes" approach).
- **Notable actors** (heads of large polities/firms in the modern era): optionally
  LLM-in-the-loop at low frequency (yearly decisions, few thousand actors), cached and
  seeded for reproducibility. Strictly optional and ablatable.

## 7.3 Emulators (surrogates)

- Neural emulators of expensive submodels (crop yield vs climate, regional climate
  downscaling, macro sub-economies) — as ML weather models emulate NWP.
- Whole-simulator emulators: map (θ, policy) → summary statistics, for history
  matching, SBI and policy search. Active learning chooses which full runs to do next.
- Emulators are always validated against fresh full-simulator runs; never the final word.

## 7.4 Differentiable simulation

- Where dynamics are smooth enough (demography, epidemiology, aggregate markets),
  JAX re-implementations or relaxations give gradients for calibration.
- Discrete agent decisions use straight-through / Gumbel-softmax style relaxations
  only inside surrogates; the authoritative engine stays discrete and deterministic.

## 7.5 Development

- The codebase is written largely with AI coding agents; the design therefore favors
  strong typing (Rust), small well-specified modules, exhaustive tests, and
  machine-checkable invariants (conservation, determinism) so agent-written code is
  verifiable.
