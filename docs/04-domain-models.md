# 4. Domain models

Each domain is a set of mechanism modules. For contested questions we keep **several
competing modules** and let data adjudicate. Below: what each domain covers, how it
enters the model, and the starting mechanisms.

## 4.1 Earth system (mostly exogenous forcing, later partly coupled)

- **Climate:** temperature, precipitation, seasonality, variability from paleoclimate
  reconstructions / transient simulations, downscaled to the grid. Holocene events
  (Younger Dryas, 8.2 ka, 4.2 ka events, Little Ice Age) come from data, not scripts.
  *Implemented (M1):* CHELSA-TraCE21k century means (ADR-0042), with the African Humid
  Period's rain (ADR-0084) and decadal anomalies from a `climate_anomaly` pack: NGRIP's
  δ18O with its century-scale signal removed, one index for the region that shifts
  temperature and scales rain each decade, so the 8.2 ka event stresses foragers and
  farmers (ADR-0107, #41).
- **Sea level & coastlines:** glacial isostatic adjustment reconstructions
  (Doggerland, Sunda shelf, Persian Gulf flooding).
  *Implemented (M1):* a `coastline` pack gives each cell's land fraction per century;
  cells with none are uninhabitable. Bands start on land, fission never sends
  daughters to the sea, and the `inundation` mechanism moves bands out of cells as
  they flood (to the least crowded habitable neighbor, or they are lost). ADR-0043.
- **Volcanism & solar forcing:** eruption records (aerosol effects on harvests).
- **Hydrology:** rivers, floodplains, aquifers, irrigation potential.
  *Implemented (M1):* a static `hydrology` pack (modern proxy) with river length,
  discharge and order, floodplain and lake fractions per cell; lakes are not habitable
  (ADR-0044).
- **Soils & terrain:** fertility, erosion, salinization (feedback from irrigation).
  *Implemented (M1):* static terrain and `soils` packs (topsoil texture, organic carbon,
  pH, coarse fragments; modern proxy, ADR-0045), and a `bedrock` pack of depth to bedrock
  within 2 m (SoilGrids250m 2017): fields wear the same depth from every soil, so a thin
  soil over rock loses its share faster (ADR-0106, #24).
- **Coupling (later):** land use → regional climate/erosion; industrial emissions →
  global temperature via a reduced-complexity climate emulator (FaIR-like) post-1750.

## 4.2 Ecology & resources

- Biome productivity, wild plant/animal resources, megafauna remnants, fisheries.
  *Implemented (M1):* forager carrying capacity per cell from the climate pack (Miami
  model productivity), dry-land area and aquatic resources (rivers, lake shores,
  coasts), recomputed as the climate changes (`oikos-sim::environment`, ADR-0047).
  The forcing's climate model has no African Humid Period, so a `humid_period` pack adds
  the rain reconstructions give the Eastern Sahara at its height, scaled and timed by
  parameters the records constrain (ADR-0084); its fall-off over Egypt is fitted to the
  Western Desert's own estimates, from its sites' charcoal and fauna (ADR-0093).
- **Domesticable species pool** by region (wheat, barley, lentils, goats, sheep,
  cattle, pigs, rice, maize…), with suitability from agro-climatic models.
  *Implemented (M1, sheep and goats):* bands manage caprines as a second practice within
  one effort budget with cultivation, where wild herds range (Asia's uplands with enough
  rain) and, with domestic animals, on pasture beyond; herding is learned from neighbors
  as cultivation is, herds become domestic by selection where wild animals live, and
  feed people on land that grows no crops (ADR-0087). Deserts too dry for open grazing
  still feed flocks along their wadis, which collect runoff from the land around them;
  and need can drive herding as it drives cultivation, through domestic stock, so
  crowded herders specialize where nothing else feeds them, though the M1 record keeps
  that hypothesis off for now (ADR-0092). Two more hypotheses wait for a profile, both
  off (ADR-0113, #51): animals pass along kinship, marriage and friendship ties as seed
  does, gifts and loans that pool the risk of losing a herd, so bands beside herders
  can start herds of their own where no wild animals lived; and floods water grazing on
  the land a river floods. Egypt's first domestic animals, among the desert hunters of
  the 6th millennium BCE, are the record that weighs them.
- Domestication as a co-evolutionary process (selection intensity × cultivation
  effort × time), not a switch.
  *Implemented (M1):* bands carry cultivation effort and the domestic share of their
  crop; effort moves toward a target nesting three competing drivers (population
  pressure, climate stress, wild-cereal abundance) as weights; cultivation selects for
  domestic traits against wild gene flow, slowly or quickly by the selection
  hypothesis; capacity blends foraging and farming by effort, so domestic crops feed
  more people and the transition can ratchet (`mechanisms::Cultivation`,
  `environment::cereal_suitability`, ADR-0049). A fourth, nested hypothesis scales the
  target by climate reliability (stable-climate hypothesis, ADR-0054; weight 7.2 in the
  calibrated defaults, ADR-0081). A fifth, nested hypothesis has bands learn cultivation
  mostly from the people around them (innovation share below 1): it then spreads as an
  innovation does, slowly while few plant and then fast (Bass 1969, Henrich 2001;
  ADR-0065; innovation share 1.6e-5 and adoption rate 0.028 in the calibrated defaults).
  Bands learn from the people around their own country too and copy what visibly pays
  (ADR-0076): each neighboring cell's people count `neighbor_learning` (0.1) as much as
  the band's own cell's, and each farmer counts farming's payoff, what a unit of effort
  feeds against foraging, to the power `success_bias` (1: linear payoff weighting; 0.23 in
  the calibrated defaults, which no record yet identifies, ADR-0080). A band copies the practice it moves toward, foragers' when
  it gives cultivation up. Wider learning evens practice out across a landscape;
  success bias speeds cultivation where it pays and holds it back where wild-cereal
  cultivation feeds fewer than foraging. Both 0 nest the learning before.
  Where rivers flood their plains, fields need no rain (ADR-0085): floodplains grow
  domestic crops only, sown by bands that have them, and their silt does not wear down,
  so the Nile and the Mesopotamian alluvium can be farmed where rain grows no cereals.
  Fields wear the soil down and yields fall once it thins below a critical share (land
  degradation, ADR-0055; 3.4e-4 per year at full effort in the calibrated defaults,
  against soil formation 7.1e-4).
  Bands sow emmer and einkorn with their crop (ADR-0101, #38): the same fields and effort,
  each wheat selected at its own fraction of the crop's selection (0.74 for emmer, from
  the early rates of Allaby et al. 2017; 0.9 for einkorn, calibrated, as the early
  rates' 0.14 leaves it wild where the record has it domestic), standing
  for how each was harvested; the crop's share is barley's, which stands for the package
  in carrying capacity, and the wheats' are judged against their own rachis records.
  Seed exchange (ADR-0102, #48; 0.3 per period, calibrated by round 14, ADR-0105): seed moves along kinship,
  marriage and exchange ties, so a band replaces a share of its seed stock, in
  proportion to its effort, with seed at the domestic shares sown by those it learns
  from; villages where few farm can then sow crops bred elsewhere.
  Where wild emmer and wild einkorn grow (ADR-0108, #38) comes from GBIF's occurrence
  records, a `wild_wheats` pack: gene flow from wild stands pulls each wheat back only
  within its ancestor's range, so seed sown beyond it, as on the Damascus basin's plain or
  the central Anatolian plateau, keeps its traits (Willcox 2005), and the rachis record
  counts wild ears gathered only there.
  Wild-cereal suitability comes from temperature and precipitation, one for every crop;
  pulses, soils and irrigation are not yet included.
- Minerals and energy: obsidian, flint, copper, tin, iron, salt, timber, later coal,
  oil, gas, uranium, renewables potential.

## 4.3 Demography & genetics

- Age-structured fertility/mortality conditional on nutrition, disease, workload,
  sedentism (Neolithic demographic transition), and later the modern demographic
  transition (education, child mortality, contraception, urbanization).
  *Implemented (M1):* logistic growth whose intrinsic rate moves with a band's
  cultivation effort from the mobile foragers' rate to the sedentary cultivators'
  (`mechanisms::Growth`, ADR-0053), the net effect of the Neolithic demographic
  transition; age structure is not yet modelled. Equal rates nest the
  no-transition hypothesis. The M1 records do not identify the rates (ADR-0067):
  populations follow carrying capacity, which cultivation raises far slower than any
  band grows, so the transition in M1 worlds is in capacity. Age structure and the
  juvenile index of rural cemeteries would test it (#33). Births and deaths are gross
  events (ADR-0068): births at the band's fertility, which moves with cultivation
  effort from `fertility` to `sedentary_fertility` (equal by default), deaths the rest
  of the logistic change.
- Kinship and marriage rules (emergent norms: endogamy, polygyny, dowry/brideprice).
- Optional lightweight population genetics (lactase persistence, disease resistance,
  ancestry tracers) — chiefly as a **validation signal** against ancient DNA.

## 4.4 Cognition & behavior (psychology, neuroscience, behavioral economics)

Not brain simulation. Agents implement validated regularities:
- Bounded rationality, satisficing, heuristics; prospect-theory risk preferences;
  present bias; loss aversion.
- **Social learning** (conformity, prestige and success bias) — dual-inheritance
  theory (Boyd & Richerson).
  *Implemented (M1):* cultivation is learned from the people of a band's cell and its
  neighbors, frequency-weighted (ADR-0065) and payoff-biased (ADR-0076; McElreath et al.
  2008). Conformist and prestige bias toward individuals, teaching, and
  in-group bias across cultural boundaries are left out.
- Cognitive limits on group size and coordination (Dunbar-type constraints).
- Norms, reciprocity, fairness, punishment; in-group/out-group bias; trust.
- Emotion-like state modulators (fear, grievance, hope) driving migration, revolt.
- Implementation: per-archetype **policy functions** (parametric or small neural)
  shared across agents; parameters calibrated, priors from experimental literature
  (doc 7 covers LLM-assisted archetype design).

## 4.5 Economy

- Production: foraging → horticulture → agriculture → pastoralism → crafts →
  manufacturing → services; production functions conditioned on technology and
  institutions.
- **Exchange emerges:** reciprocity/gift → barter → commodity money → coinage →
  credit → banking → financial markets. Money arises where trade frequency and
  double-coincidence costs make it advantageous (search-theoretic models of money).
  *Implemented (M1, ADR-0060):* down-the-line exchange of obsidian as a tracer of
  contact (`oikos-sim::exchange`, Renfrew 1977): every year, people within a day's walk
  of a source group's volcanoes have its obsidian, and every other occupied cell holds
  the mean of itself and its occupied neighbors less a share used up or kept
  (`obsidian_loss`), so availability falls off about exponentially along settled land
  and never crosses empty land. It changes nothing else yet; goods with value, and
  exchange that responds to them, come later. Sourced assemblages constrain the loss:
  0.0095 in the calibrated defaults, an e-fold about every 75 km (ADR-0061, ADR-0081).
- Markets: local price formation via decentralized bargaining/matching; routes with
  transport costs by terrain, technology (pack animals, ships, rail, containers).
- Specialization and division of labor tied to market size (Smith) and urbanization.
- Modern era: firms, labor markets, credit cycles, government budgets, trade,
  global value chains — informed by macro ABMs (e.g. EURACE, Axtell's firm model).

## 4.6 Institutions & power (political economy — first class)

- Groups form, merge and split by cooperative benefit vs. coordination cost.
  *Implemented (M1):* fission by scalar stress (`mechanisms::Fission`, ADR-0070): a
  group's size limit rises with its cultivation effort from foragers' camps (60) to
  farming villages (186 in the calibrated defaults, ADR-0081; Hutterite colonies split
  at 127, Alberti 2014), and a group above it splits, founding a new
  settlement in its own cell while the land has room. Farming groups gather into the
  largest village of their cell that has room (`mechanisms::Aggregation`, ADR-0071):
  sedentism, defence, exchange, marriage and ritual draw them together, up to the limit
  of face-to-face coordination. A settlement may hold several groups (ADR-0082): each
  group keeps its own limit, and institutions that bind households across groups
  (ritual, shared buildings, descent; `settlement_integration`) and leaders whose
  decisions others accept, which wealth inequality allows (`settlement_hierarchy`),
  hold a settlement of several groups together up to its own limit. A group too large
  to merge into a village lives beside it as a neighbourhood of its own, a farmer's
  daughter stays in her settlement while it has room (foragers' daughters camp alone), a settlement over its limit sheds its
  smallest group, and the endemic infection (ADR-0075) persists in a settlement as in
  one group of all its people. Surveys count settlements, not groups.
  Land and settlement are counted per catchment, not per cell (#50, ADR-0112): farmers
  worked fields within about an hour's walk of home and foragers ranged within about two
  (Vita-Finzi et al. 1970), so what a place feeds, how crowded it is and which village a
  farmer joins are reckoned over the land within `catchment_radius_km` (8 km): a block
  of cells, the coarser H3 cell nearest that circle in area. A village draws its farmers
  from its whole catchment, groups that leave found new homes in a neighboring one, and
  water at hand anywhere in it feeds fishers on all its land. At resolution 5 each cell
  is its own catchment; on a finer grid the same parameters give villages and
  populations of about the same size, and the grid adds where in a catchment people live.
- Wealth inequality. *Implemented (M1):* each group carries the spread of its households'
  wealth (the variance of log wealth; `mechanisms::Inequality`, ADR-0074), after the
  intergenerational model of Borgerhoff Mulder et al. (2009): a share of wealth passes
  from parents to children, and chance shocks add to it. Its drivers are social: where
  land is open, sharing and levelling norms keep wealth from being inherited (foragers
  and horticulturalists, Gini about 0.25); where farmland is scarce and closed in by
  other farmers, sea or desert, it becomes property and is inherited (agriculturalists,
  about 0.48; Demsetz 1967, Carneiro 1970, Bowles and Choi 2013). Herds are wealth
  on open land too (#45): animals are owned, inherited and given as bridewealth, and
  drought, epizootics and raids shake herders' fortunes (pastoralists, about 0.42). The
  three economies mix by a group's effort: foraging, cultivating owned land, herding. It feeds back on
  nothing yet: status competition, patronage, inequality's effect on fertility, fission
  and conflict, and the spread between merged groups' mean wealth are left out.
- **Polities:** chiefdoms → states → empires, with state capacity (taxation,
  bureaucracy, coercion), legitimacy, and elite structure.
- Competing state-formation modules: circumscription (Carneiro), military technology
  diffusion and inter-polity competition (Turchin et al. 2013), irrigation/managerial,
  surplus/storage-driven inequality.
- **Elites & rent-seeking:** extractive vs inclusive institutions (Acemoglu &
  Robinson), elite overproduction and secular cycles (Turchin), collective action
  (Olson), path dependence (North).
- Law, property rights, religion/ideology as legitimacy and coordination technology.
- Policy adoption is endogenous: who has power to adopt, block or subvert it.

## 4.7 Knowledge, technology & culture

- Technologies as nodes in a **data-defined capability graph** (prerequisites,
  resource needs, effects) with *combinatorial* discovery probability rising with
  population connectivity, specialization and existing knowledge (Kremer; Henrich's
  collective brain). Bounded open-endedness: discoveries draw from the graph, but
  timing, place, diffusion and loss (e.g. Tasmania effect) are emergent.
- Cultural traits (norms, beliefs, languages, religions) as transmissible variants with
  social-learning dynamics; language families as validation targets.
- Writing, record keeping, science and education as multipliers on transmission
  fidelity and innovation rate.

## 4.8 Disease & health

- Compartmental/agent epidemiology at settlement and network level; zoonotic spillover
  linked to domestication and density; endemic vs epidemic regimes; trade-route
  pandemics (Antonine, Justinian, Black Death, Columbian exchange).
  *Implemented (M1):* an endemic infection within groups (`mechanisms::Disease`,
  ADR-0075), the burden settlement adds. Transmission rises with group size, so the infection persists only above a
  critical size and then at prevalence 1 − 1/R0 (Anderson and May 1991; Bartlett 1957),
  killing `disease_mortality` of the infected each year. Mobility cuts transmission:
  foragers move camp away from waste, vermin and the sick, acting on disgust (the
  behavioral immune system, Schaller and Park 2011), while villagers stay with their
  fields and stores. So a group's critical size falls with its cultivation effort from
  125 to 50 people: foragers' camps add none (their own persistent infections, endemic
  even in isolated bands, Black 1975, are in their baseline mortality), and the large
  villages that aggregation draws people into pay for their size (Eshed et al. 2010). Avoidance
  changing where people live, hygiene norms, zoonoses from herds, epidemics between
  villages and age-specific mortality are left out.
- Nutrition, sanitation, medicine (germ theory, vaccines, antibiotics) as technologies.
- Modern health systems and disease burden calibrated to GBD data.

## 4.9 Conflict & migration

- Raiding, warfare (technology, logistics, terrain), conquest, refugee flows.
- Migration driven by expected utility differentials, networks, and push factors
  (climate shocks, famine, violence).
  *Implemented (M1):* forager dispersal: crowded cells bud off groups into less
  crowded neighbors with room, the wave of advance by which foragers fill the region
  (`mechanisms::Dispersal`, ADR-0048). Migrants, dispersing or daughters leaving a
  full cell, judge a destination by what its land would feed with them there, their
  practice pooled with its residents' (#45): herders look for grazing, so they move onto
  pasture where foragers find nothing, and nobody moves where neither forage nor pasture
  can feed them.

## 4.10 Modern-era additions (1800 →)

- Industrialization (energy, machines, factories), the demographic transition,
  urbanization, mass education, nation-states, colonialism and decolonization,
  globalization, digital networks and **AI/automation** as a technology with labor-
  market, productivity and power-concentration effects.

## 4.11 Welfare indicators (computed everywhere, all eras)

Health & longevity · material wellbeing (consumption, poverty, inequality) ·
capabilities & freedom (education, agency, violence, political freedom) ·
sustainability (resource stocks, emissions, ecological integrity). See doc 6.
