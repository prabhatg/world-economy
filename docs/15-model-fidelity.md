# 15. Model fidelity plan

What the model needs next to describe the past more faithfully, in what order, and how
we will know each step helped. Every item is a GitHub issue labeled `fidelity` under its
milestone; this document says why each matters and how the items depend on each other.
The [roadmap](10-roadmap.md) tracks milestones, [ADRs](adr/README.md) the decisions
taken, and [open questions](open-questions.md) what is still unresolved.

## 15.1 What the calibration rounds taught us

Calibration rounds 9 and 10 (ADR-0073, ADR-0077) history matched the M1 model against
five records. The fit is good: the calibrated defaults pass every target. But the
records leave many parameters unconstrained:

- **Village size and aggregation** (`village_fission_size`, `aggregation_rate`): no
  record saw settlement size until F10 (below); it now does, loosely, and finds the
  model's villages smaller and less growing than the record's. Settlements of several
  groups (F7, ADR-0082) can reach the record's size, not its growth; the record wants
  more than one group's worth, not how many (ADR-0083).
- **Pace of learning** (`success_bias`, `adoption_rate`, `innovation_share`): the
  records see when cultivation took hold, which these trade off against each other;
  where it took hold first (F3, ADR-0080) does not separate them either.
- **Vital rates** (`fertility`, `sedentary_growth_rate`): populations follow carrying
  capacity, and the records measure stocks, not births and deaths.
- **Disease** (`disease_threshold`, `disease_mortality`): seen only through its effect
  on the number of villages.
- **Wealth** (four parameters): the house-size record was left out of round 10's first
  waves; judged in its fourth, it leaves all four spread over their priors.

Fidelity therefore means two things: **records fine-grained enough to tell competing
explanations apart**, and **mechanisms fine-grained enough to produce what those
records measure**. A mechanism no record can test adds parameters, not knowledge, so
each mechanism item below names the record that will judge it.

## 15.2 Principles

1. **Record before mechanism.** Add a record that separates hypotheses before adding
   detail it cannot judge.
2. **One identifiable step at a time.** Each item ends with a calibration round or a
   profile showing what it constrains. An item that constrains nothing is reported as
   such, not tuned until it does.
3. **Psychological and social drivers are explicit** (the standing rule), and each
   mechanism's ADR records what it leaves out.
4. **Licences first.** Store manifests and ingestion code, not restricted data. A record
   whose values cannot be published is left out of committed run files (`--leave-out`,
   ADR-0077).
5. **Measured cost.** Finer resolution is benchmarked before it is adopted, because the
   calibration budget is runs × cost per run.

## 15.3 The plan

Ordered by value for identification against cost. Items in the same tier can run in
parallel.

| Tier | Item | Issue | What it identifies | Depends on |
|---|---|---|---|---|
| 1 | F10 Village sizes from survey site areas (done, ADR-0078: constrains no parameter alone; the model's villages are too small and too level, which motivates F7) | [#34](https://github.com/prabhatg/world-economy/issues/34) | village size, aggregation, disease's critical size | – |
| 1 | F2 Regional radiocarbon by subregion (done, ADR-0079: narrows no parameter alone; shows the periphery filling too early and the core too late, open question 37, and rules out the old defaults, ADR-0081) | [#35](https://github.com/prabhatg/world-economy/issues/35) | dispersal, neighbor learning, regional timing | – |
| 1 | F3 Spatial onset of cultivation (done, ADR-0080: constrains nothing new; the contrast follows the abundance weight, not success bias) | [#36](https://github.com/prabhatg/world-economy/issues/36) | success bias against adoption rate | – |
| 1 | F1 House-size Ginis: judged in run files (decided 2026-11-03; open until the authors reply) | [#43](https://github.com/prabhatg/world-economy/issues/43) | wealth transmission and shocks | – |
| 2 | F11 Juvenile index (age structure) | [#33](https://github.com/prabhatg/world-economy/issues/33) | fertility, the demographic transition | Guerrero et al. 2008 chapter |
| 2 | F4 Disease lesions before and after farming (done, ADR-0086: sees the rise in infection with farming, leaning toward foragers who spread less of it; not the mortality) | [#42](https://github.com/prabhatg/world-economy/issues/42) | disease mortality | – |
| 2 | F7 Settlements of several groups (towns; done, ADR-0082, ADR-0083: integration calibrated to four groups' worth, hierarchy not identified; villages larger, though below the record's size and level through time, open question 38) | [#39](https://github.com/prabhatg/world-economy/issues/39) | the largest sites | F10 |
| 2 | F13 Northeast Africa: the African Humid Period in the forcing and Nile-valley farming, so the subregion's radiocarbon can be judged (Humid Period in, ADR-0084, floodplain farming, ADR-0085, herders' route across Sinai, ADR-0092, and the Humid Period's rain over Egypt from the Western Desert's estimates, ADR-0093, which settles the desert and brings the 25% and 50% dates to the record's; judged since ADR-0095, with the Delta's early foragers out of sight on drowned land and under its silt; lakes a modern work filled out of the hydrology pack since ADR-0098; open question 40 keeps the valley's Humid Period population) | [#44](https://github.com/prabhatg/world-economy/issues/44) | dispersal into the Green Sahara and its retreat | F2; F5, F6 for the Nile |
| 3 | F5 Herding: sheep and goats (done, ADR-0087, ADR-0088, ADR-0090: management and selection calibrated in round 11; herd wealth and herders' movement follow in [#45](https://github.com/prabhatg/world-economy/issues/45)) | [#37](https://github.com/prabhatg/world-economy/issues/37) | management, mobility, wealth in herds | faunal record |
| 3 | F5 Herders' wealth and movement: herds as inheritable wealth, migrants judging land by what they would graze (done, ADR-0089, ADR-0091: herd wealth not identified by the M1 record, its literature values kept; herders still stop at Sinai, which has no grazing, open question 40, #44) | [#45](https://github.com/prabhatg/world-economy/issues/45) | pastoral wealth transmission, herders' spread | F5 herding |
| 3 | F6 Crops beyond barley (emmer and einkorn in, ADR-0101: sown with the crop, each selected at its own pace and judged against its own rachis record, einkorn's pace calibrated; their ancestors' ranges, pulses and the crop mix in capacity remain) | [#38](https://github.com/prabhatg/world-economy/issues/38) | per-crop domestication | per-crop records |
| 3 | F14 Springs, oases and seed exchange: farming villages of the desert margins (El Kowm 2, Wadi Fidan, Jilat), whose crops were domestic while the model's few plots there stay wild (seed exchange in as a hypothesis off by default, ADR-0102; wild cereal gathered beside the fields counted in each assemblage, ADR-0103; springs remain) | [#48](https://github.com/prabhatg/world-economy/issues/48) | seed exchange along social networks, farming beyond rainfall | a springs dataset |
| 3 | F12 Soil depth | [#24](https://github.com/prabhatg/world-economy/issues/24) | soil degradation | – |
| 4 | F8 Finer grid (H3 resolution 6) | [#40](https://github.com/prabhatg/world-economy/issues/40) | local ecology, site catchments | benchmark, [#23](https://github.com/prabhatg/world-economy/issues/23) |
| 4 | F9 Sub-century climate | [#41](https://github.com/prabhatg/world-economy/issues/41) | climate-stress hypotheses | climate records |

**Tier 1** adds records, reuses data the project already ingests, and attacks the
largest unconstrained groups. **Tier 2** needs data still to be obtained or a mechanism
that tier 1 shows is missing. **Tier 3** widens the economy (animals, crops) and
**tier 4** the resolution; both multiply parameters or cost, so they wait until the
records can judge them.

## 15.4 How an item is done

1. Its issue states the record, the operator, the parameters it should constrain, and
   the licence check.
2. TDD as everywhere ([14](14-engineering-practices.md)): operator unit tests on
   synthetic data, a test on the real record, and a stylized-fact test for a mechanism.
3. An ADR row, an update to [04](04-domain-models.md) or [05](05-calibration-assimilation.md),
   the roadmap paragraph, and the open question it answers or raises.
4. A calibration round or profile, its files in `calibration/runs/`, and the result
   (constrained or not) in the ADR and in this plan's table.
