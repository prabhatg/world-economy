# 7. AI integration

AI is used where it multiplies human-scale effort: reading the literature, giving
simulated people realistic behavior, standing in the modern world as actors of their
own, making expensive simulations cheap enough to calibrate and search, and helping
people understand millions of simulated worlds. It is **not** a per-agent runtime
brain for billions of agents, and it never decides what history the model must
reproduce ([ADR-0062](adr/README.md)).

Two constraints shape every use inside a run (§7.6): an AI that has read history must
not leak its outcome into a hindcast, and anything a volunteer's machine runs must give
bit-identical results so it can be validated by hash.

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

## 7.2 Behavior: AI as the source of simulated people's choices

Hand-written decision rules are the weakest part of agent-based models. Models trained
on human choice data now predict behavior well: a model fine-tuned on 160 psychology
experiments captures held-out participants' choices better than the cognitive models
built for them (Binz et al. 2025), and LLM agents built from two-hour interviews
predict the held-out survey answers of the people they model with 83-86% of the
consistency those people show with their own answers two weeks later (Park et al.
2024). This is how psychology, behavioral economics
and neuroscience enter the model: through what they establish about choice (time and
risk preference, social learning, norms, stress and scarcity, how nutrition, disease
and toxins shape cognitive development), not by simulating neurons.

- **Archetypes.** LLMs help design archetypes (forager band, Neolithic farmer
  household, Bronze Age elite, medieval peasant, merchant, industrial worker, modern
  smallholder, urban informal worker, bureaucrat, politician…) grounded in
  ethnographic and behavioral evidence.
- **Distilled policies.** A behavior model answers sampled situations from the
  simulation ("a household with this harvest, these neighbors and these prices: does
  it move, plant, store, trade?"); its answers are distilled into a compact policy
  network per archetype, shared by every agent of that archetype and batched on CPU or
  GPU (cf. AgentTorch's LLM archetypes). The network's inputs are the agent's sensed
  state; its few free parameters (temperature, weights on payoffs, norms) stay in the
  registry with priors and are calibrated against the record like any mechanism. The
  behavior model gives the prior; history judges it.
- **Integer inference.** Policy networks run in quantized integer arithmetic (int8
  weights, int32 accumulation, fixed-point activations by table), which is exact on
  every CPU, GPU and the browser, so they obey the reproducibility contract (doc 3
  §3.5) and run on volunteer GPUs with hash validation.
- **Plural behavior.** Competing behavior models (rational choice, bounded
  rationality, a learned policy) are swappable mechanism modules judged by fit, like
  any competing theory (doc 1, principle 3).

## 7.2a Actors: AI systems in the simulated world

From about 2020 the world being simulated contains AI itself, and the policy lab's
questions (who gains from automation, whether power concentrates, how fast
capabilities spread) depend on how it is modeled. AI enters the modern era as a
**class of agents and capital**, not only a productivity parameter:

- Task-based production (Acemoglu & Restrepo 2018, 2019): AI takes over tasks from
  workers at a cost that falls with compute and capability, and creates new tasks;
  wages, employment and the labor share follow from which tasks move.
- AI systems as agents with owners, costs and capabilities: they do work for firms and
  states, advise decisions and act in markets, so ownership, concentration and
  governance become mechanisms with consequences.
- Capability and compute growth, diffusion between firms and countries, and policy
  levers (taxes on automation, compute governance, public AI) are scenario inputs,
  with ensembles over the deep uncertainty (open question 8).
- **Notable actors** (heads of large polities and firms in the modern era) may be
  LLM-in-the-loop at low frequency (yearly decisions, a few thousand actors). Their
  decisions are recorded once into a content-addressed **decision pack** that work
  units replay like a forcing pack (doc 3 §3.8a), so volunteers reproduce runs without
  running an LLM. Strictly optional and ablatable.

## 7.2b Analysis: AI that helps people understand ensembles

With millions of worlds the bottleneck is understanding them. AI over the analysis
store (doc 11) can group worlds into a few distinct pathways, find what separates the
worlds where people fare better from the rest (scenario discovery; cf. PRIM in
decision making under deep uncertainty), answer questions in plain language with the
runs as evidence ("why did these worlds avoid the famine?"), and narrate one world's
history from its events, which is also how a volunteer meets the world their computer
ran. Every claim links to the runs and summaries behind it; narratives say they are
narratives of one simulated world, not predictions.

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

## 7.5 Development and science

- The codebase is written largely with AI coding agents; the design therefore favors
  strong typing (Rust), small well-specified modules, exhaustive tests, and
  machine-checkable invariants (conservation, determinism) so agent-written code is
  verifiable.
- AI agents also do science here: they ingest datasets, propose mechanisms and
  observation operators, run calibration waves and write up what the record rules
  out, with every step recorded (runs, ADRs) so people can check it.

## 7.6 Constraints on AI inside a run

1. **No leakage into hindcasts.** An LLM trained on text about history knows how it
   went; deciding agents' choices with it would steer a world toward what happened and
   make the fit to history meaningless. Behavior models used before the present are
   trained on experiments, surveys and ethnography, not on narratives of the period;
   their effect is tested on held-out periods and regions (doc 1, "held-out
   validation"); full LLMs act directly only in scenarios after the twin's start
   (2026), where there is no answer to leak and where reactions to policies never
   tried are what we most need.
2. **Bit-identical or replayed.** Anything inside a work unit is either integer
   inference (policy networks) or replayed from a decision pack; nothing calls a model
   service during a volunteer run.
3. **Ablatable.** Every AI component can be switched off or replaced by a simple rule,
   and results report how much they depend on it.
4. **Calibrated, not trusted.** AI-derived behavior has priors and free parameters like
   any mechanism; where the record contradicts it, the record wins.

## References

- Acemoglu, D., Restrepo, P. (2018). The race between man and machine: implications of
  technology for growth, factor shares, and employment. *American Economic Review*
  108, 1488-1542. doi:10.1257/aer.20160696
- Acemoglu, D., Restrepo, P. (2019). Automation and new tasks: how technology displaces
  and reinstates labor. *Journal of Economic Perspectives* 33(2), 3-30.
  doi:10.1257/jep.33.2.3
- Binz, M., Akata, E., Bethge, M., et al. (2025). A foundation model to predict and
  capture human cognition. *Nature* 644, 1002-1009. doi:10.1038/s41586-025-09215-4
- Park, J.S., Zou, C.Q., Kamphorst, J., et al. (2024). LLM agents grounded in
  self-reports enable general-purpose simulation of individuals (first titled
  "Generative agent simulations of 1,000 people"). arXiv:2411.10109
