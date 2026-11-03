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
    7,400-6,600 BCE, when half the assemblages are wholly domestic. History matching
    (ADR-0052) finds worlds that fit both ends, but they are rare: of the worlds wild
    enough before 8,000 BCE, a third reach the record's share after 7,000 BCE, and the
    consistent ones sit at its low end (median 0.65 against 0.80). The stable-climate
    hypothesis is implemented (ADR-0054) but century-resolution climate cannot test
    it: suitability saturates in the cereal belt's core, where cultivation ignites.
    Delaying the onset to 10,500 BCE by fiat fits the first two radiocarbon eras but
    not the rest (ADR-0054), so onset timing is part of the gap, not all of it.
25. What the radiocarbon rise measures (ADR-0050): binned, taphonomy-corrected date
    mass in M1 rises about fifteenfold from 11,500 to 6,000 BCE and falls after; the
    default world's activity rises about threefold and keeps rising. Is the gap
    demographic (a Neolithic demographic transition, #14), visibility (sedentary
    villages leave far more datable material, #13), or research bias toward the
    Neolithic? Date density alone cannot separate them; site counts and sizes can
    (#17). The operator also conditions out where dates are dense, so it needs a
    research-intensity model or site surveys to use spatial density. In history
    matching (ADR-0052) the gap persists across the prior space - worlds consistent with
    every summary rise about half as much in log terms as the record before 9,900 BCE -
    but stays inside the radiocarbon tolerance; low growth rates, filling the region
    slowly, come closest. A Neolithic demographic transition does not close it
    (ADR-0053): populations sit at carrying capacity, so the rise is set by capacity,
    which doubles by 11,500 BCE through wild-cereal cultivation (question 24). The
    record also falls after 5,100 BCE while every world keeps rising: a Neolithic
    boom and bust (Shennan et al. 2013), the 8.2 ka event, or research focus on the
    Neolithic? Land degradation (ADR-0055) can lower capacity after the boom, and an
    imposed decline improves the radiocarbon fit by 24-44 log-units; whether erosion
    at plausible rates, a later onset and faster selection reproduce the whole record
    together is for the next calibration wave. Round 2 (ADR-0056): the best-fitting
    consistent worlds erode at 1.1-1.6e-4 per year and do rise and fall, but by about
    half the record's amplitude. Site counts (ADR-0059) contradict a region-wide bust:
    Upper Mesopotamia's survey sites multiply into the Late Chalcolithic, and worlds
    consistent with both records have almost none (the calibrated default keeps
    rising). Is the radiocarbon fall southern Levantine (surveys there are not yet in
    the record) or research focus on the Neolithic?
26. How large is the M1 model discrepancy (ADR-0052)? The first waves used 0.25 on
    radiocarbon era log shares and 0.1 on domestic shares, and at those values the
    radiocarbon record rules out nothing. Tighter values would make it bind but must
    be justified independently (e.g. by held-out regions or by a second dates
    database), not chosen to make it bind. With 800-year eras judged together
    (ADR-0056) the record binds at the same discrepancy.
27. Does cultivation speed population growth in M1 (ADR-0053, ADR-0057, ADR-0059)? Round 2 did not
    emulate `sedentary_growth_rate`; the calibrated defaults set it equal to the forager
    rate (no transition), yet the record mildly prefers slower sedentary growth (-3,990
    against -3,997 on radiocarbon at 0.01). Round 3 (ADR-0058) emulated it: the record
    does not identify it (consistent worlds score alike from under 0.7 to over 1.4 times
    the forager rate). The round-4 default has one (sedentary growth 2.7 times the
    forager rate), but as one world among many, not as evidence. Round 5's best world
    (ADR-0061) fits the four records as well with sedentary growth 0.4 times the
    forager rate. The rural-cemetery juvenile index (Bocquet-Appel 2002) could test
    it directly.
    Answered (ADR-0067): not with these records. Populations follow carrying capacity,
    which at the defaults grows nearly 20 times slower than a band's growth rate, even
    with sedentary growth at the prior's floor. The rates only set a lag of a few
    percent, and the records measure stocks, not vital rates.
    The 534 consistent worlds of waves 7-10 keep the ratio's prior distribution, and a
    profile at the defaults from 0.1 to 4 times the forager rate is flat. In M1 worlds
    the transition is in capacity. Testing the rates needs age structure and the
    juvenile index (#33). The model side is built (ADR-0068): fertility moves with
    cultivation and a juvenile-index operator reads it; the Levant record is to come.
28. How did obsidian move beyond the settled chain (ADR-0060, ADR-0061)? Down-the-line
    exchange passes obsidian only between neighboring settled cells, so Cyprus, supplied
    with Cappadocian obsidian by boat, is never reached and its assemblages are dropped.
    Both sources supply equally, though Göllü Dağ's obsidian may have been preferred, and
    the record's eastern obsidian moves west after 6,900 BCE (into the Pottery Neolithic
    and Halaf), which a fixed loss per step produces only through where people settle.
    Sea crossings, source preference and directional exchange would each add a
    parameter; the zones say whether they are needed.

29. Do behavior models trained on today's participants transfer to the past
    (ADR-0062)? Experiment and survey data come mostly from modern, often Western,
    participants; foragers and early farmers lived under other norms and constraints.
    Distilled policies may need archetype-specific priors from ethnography, and their
    contribution must be tested on held-out periods and regions before it is trusted.

30. When did people begin to plant (ADR-0063)? Every world consistent with the four
    M1 records gets 52-96% of its food from planted crops by 11,500 BCE (the viewer
    ensemble), while cultivation of wild cereals is first seen in the Pre-Pottery
    Neolithic A, about 10,000 BCE, and at a small share. None of the records judged
    dates the onset of cultivation: rachis dates domestication, the surveys start at
    6,500 BCE, and radiocarbon and obsidian say nothing of diet. An onset operator
    (archaeobotanical evidence of pre-domestication cultivation, such as arable weed
    floras, or the share of cereals among plant remains) would rule these worlds out.
    Partly resolved (ADR-0064): the share of crops among charred plant remains, anchored
    on the Pottery Neolithic, implies an effort of 0.09 before 10,000 BCE and rules out
    worlds planting most of their food then; the calibrated default puts in 0.34. The
    earliest era rests on four phases at three sites, and the conclusion on a model
    discrepancy of 0.1 (at 0.2 the earlier defaults would pass).

31. Why does no M1 world plant more over time (ADR-0064, #30)? The onset record's
    implied effort rises from 0.09 before 10,000 BCE to about 0.6 in the PPNB; in wave
    6 no world rises by more than 0.22, and consistent worlds pass by planting about a
    third throughout. Cultivation effort moves toward a target set by pressure, stress
    and abundance, which it reaches early. Yields that improve with domestication,
    accumulated knowledge spread by social learning, or storage and sedentism could
    each make planting grow; each would need a prior and a nested test.
    Resolved (ADR-0065): with cultivation learned mostly from the people around them
    (innovation share about 10⁻⁴, adoption rate 0.007), a world consistent with all five
    records puts in 0.09, 0.57, 0.75 and 0.54. Yields and storage were not needed.

32. Do the records pull apart on when planting began (ADR-0065)? The consistent world
    that plants as the onset record does fits radiocarbon, rachis and obsidian worse
    (log-likelihood -4,404 against -4,260) than the best-fitting one, which plants 0.40
    before 10,000 BCE, near the cutoff. The log-likelihood diagnostics that choose the
    defaults cover only those three records; the onset and the site counts are judged as
    targets. Either the records disagree (early planting raises populations and spreads
    domestic barley in time, late planting delays both), or the model's demography or
    selection ties them too tightly, or the ranking should weigh the onset record too.
    More waves near the late-planting worlds, and an onset log-likelihood, would tell.
    Resolved (ADR-0066): they do not pull apart. Under history matching's own error
    model (each record's summaries with observation covariance plus discrepancy), the
    consistent worlds nearest the record plant late, and the best of them (0.20, 0.55,
    0.61, 0.48) sits nearer than the early planter on four records and about as near on
    the radiocarbon eras. What preferred early planting was the per-date radiocarbon
    likelihood, which allows for no discrepancy. Defaults are now ranked by the
    summaries, and the late planter is the registry default.
33. What record sees village size and aggregation (ADR-0073)? Farmers' villages split
    by scalar stress (`village_fission_size`, ADR-0070) and settled groups join the
    largest village of their cell (`aggregation_rate`, ADR-0071), but none of the five
    M1 records responds to either: across round 9's consistent worlds both spread over
    their priors as all runs do, so the size limit stays at its literature value (127,
    Alberti 2014) and the joining rate a placeholder. Survey site areas (settlement-size
    distributions by era, as in the Anatolian and Upper Mesopotamian surveys already
    used) or excavated house counts per phase could; an operator would compare them
    with the distribution of group sizes the model's settlements reach. The same holds
    for fertility and sedentary growth until the juvenile-index record (#33) joins.
34. How strongly do houses track wealth, and when did inequality rise (ADR-0074)? The
    house-size operator takes floor area to grow as wealth to the power 0.7, a judgment
    from comparing house-size and wealth Ginis across two compilations; within-site data
    on house size and household wealth would set it. The five M1 sites' Ginis rise from
    the PPNA to the Late Chalcolithic, while the calibrated defaults' stay near 0.25-0.27
    throughout: people around every site plant from early on, so land is partly closed
    everywhere. With two to nineteen houses a site the record cannot yet tell these
    apart. More sites with measured houses (Kohler and Smith 2018 list others outside the
    supplement) would. The generation length (28 years, Fenner 2005) is to be checked
    against the paper.
35. How heavy was disease in Neolithic villages, and at what size did it persist
    (ADR-0075)? The critical size (50 sedentary people), foragers' relative contact (0.4)
    and the mortality of the infected (0.005 a year) are judgments. The records bound the
    mortality from above only indirectly: at four times the default, domestication lags
    (a domestic barley share after 7000 BCE of 0.24, against the observed 0.80). The skeletal record would test it directly:
    Eshed et al. (2010) compare lesions of infection in 200 Natufian and 205 Neolithic
    skeletons of the southern Levant, and an operator could compare prevalence before and
    after farming with the model's people-weighted prevalence. Behavioral responses
    (villages splitting or dispersing under disease, hygiene norms) are left out until a
    record calls for them.
36. How strongly did people copy farming for its payoff, and how far did they look
    (ADR-0076)? Success bias at 1 brings domestication and cultivation forward of the
    record in the calibrated world, which was fitted without it, to the edge of the
    record (implausibility 2.86-2.98), so it is 0 by default for now; the bias trades off
    against `adoption_rate` and `innovation_share`, so the three are to be calibrated
    together (round 10). The onset and rachis records may not separate a faster spread
    where farming pays from a faster spread everywhere; the spatial pattern of the onset
    (does cultivation start in the richest stands and spread outward?) could. Neighbor
    learning (0.1) changes the fit little (2.26-2.28 against 2.32-2.34): the M1 records
    do not yet see how far practice is learned.

## Policy lab
6. Default welfare weightings to show (if any) beyond Pareto fronts?
7. Which hindcast interventions form the validation suite for the policy lab?
8. How to model AI itself as a technology — productivity, labor displacement,
   power concentration — with so little historical data? ADR-0062 makes AI systems a
   class of agents and capital with task-based production; capability growth and
   diffusion stay scenario inputs with ensembles over them.
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
