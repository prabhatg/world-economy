# Open questions

## Science
1. Which phenomena must emerge vs. be allowed as data-driven triggers in Mode B
   (e.g. may assimilation "introduce" a domesticate where data says it appeared)?
2. How many competing mechanism modules per contested question can we afford?
   (Cultivation nests its competing drivers as weights of one mechanism, ADR-0049.
   The barley rachis record favours protracted over rapid domestication, ADR-0051.)
3. Model discrepancy: how do we represent "the model is structurally wrong here"
   explicitly in calibration?
4. How far to take genetics (ancestry tracers only, or selected loci)?
5. Named historical individuals in Mode B: ever, or only anonymous agents?
24. Cultivation ratchet (ADR-0049): any cultivation of rich wild stands raises carrying
    capacity, so once people overshoot forager capacity effort ratchets up. At the
    placeholder defaults cultivation begins across the Fertile Crescent by 11,800 BCE,
    earlier than the PPNA (~9,700 BCE), while domestic crops first dominate around
    9,000-8,500 BCE, close to the archaeobotanical record. Is the ratchet too strong
    (`farm_density`, `wild_yield_ratio`), is a labour cost of cultivation missing
    (Bowles 2011), or does cultivation need the stable Holocene climate (Richerson,
    Boyd and Bettinger 2001)? To be settled against radiocarbon and archaeobotanical
    timing (#17, #19). Making cultivators more visible to the radiocarbon operator
    lowers the M1 score (ADR-0050), consistent with an onset that is too early. The
    rachis record shows a second gap (ADR-0051): at the defaults the model's crop is
    1-3% domestic at PPNA sites, close to the record's 0-6%, but only 25-35% domestic in
    7,400-6,600 BCE, when half the assemblages are wholly domestic.
25. What the radiocarbon rise measures (ADR-0050): binned, taphonomy-corrected date
    mass in M1 rises about fifteenfold from 11,500 to 6,000 BCE and falls after; the
    default world's activity rises about threefold and keeps rising. Is the gap
    demographic (a Neolithic demographic transition, #14), visibility (sedentary
    villages leave far more datable material, #13), or research bias toward the
    Neolithic? Date density alone cannot separate them; site counts and sizes can
    (#17). The operator also conditions out where dates are dense, so it needs a
    research-intensity model or site surveys to use spatial density.

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
23. CHELSA-TraCE21k licence: EnviDat's record (doi:10.16904/envidat.211) gives CC BY-SA
    4.0 in its licence field and CC BY 2.0 in its description. We follow CC BY-SA 4.0,
    so the climate pack is ShareAlike, unlike our CC-BY 4.0 default for data
    (ADR-0025). Ask the dataset maintainers (WSL) which applies.
