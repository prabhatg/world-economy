# Open questions

## Science
1. Which phenomena must emerge vs. be allowed as data-driven triggers in Mode B
   (e.g. may assimilation "introduce" a domesticate where data says it appeared)?
2. How many competing mechanism modules per contested question can we afford?
3. Model discrepancy: how do we represent "the model is structurally wrong here"
   explicitly in calibration?
4. How far to take genetics (ancestry tracers only, or selected loci)?
5. Named historical individuals in Mode B: ever, or only anonymous agents?

## Policy lab
6. Default welfare weightings to show (if any) beyond Pareto fronts?
7. Which hindcast interventions form the validation suite for the policy lab?
8. How to model AI itself as a technology — productivity, labor displacement,
   power concentration — with so little historical data?
9. Who are the first external users/collaborators (academics, NGOs, policymakers)?

## Engineering
10. ~~GPU backend~~ — wgpu first (ADR-0027). Remaining: when to introduce MPI.
11. ~~Grid~~ — H3 (ADR-0009). Remaining: refinement policy.
12. Checkpoint storage budget; prefix-sharing strategy.
13. Reproducibility guarantee across CPU↔GPU: bitwise or tolerance-bounded?
14. ~~License~~ — Apache-2.0 / CC-BY 4.0 (ADR-0025).

## Volunteer computing
18. ~~Native client UI shell~~ — resolved: Flutter shell over a Rust daemon (doc 13).
    Remaining: verify app-store background-compute policies and Windows sandboxing.
19. Name accepted (ManyWorlds / Oikos, ADR-0026); trademark, domain and app-store checks still to do.
20. Which fiscal sponsor (e.g. an established open-source/science foundation)?
21. Explore-queue design: how much steering do explorers get, and can any explore
    results be reused scientifically (e.g. as importance-weighted samples)?
22. Credit formula: compute-only, or also credit verified knowledge contributions?

## Project
15. Cloud budget envelope per month.
16. Timeline expectations for M0/M1.
17. Seeking grants/HPC allocations — when, and with which collaborators?
