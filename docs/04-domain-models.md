# 4. Domain models

Each domain is a set of mechanism modules. For contested questions we keep **several
competing modules** and let data adjudicate. Below: what each domain covers, how it
enters the model, and the starting mechanisms.

## 4.1 Earth system (mostly exogenous forcing, later partly coupled)

- **Climate:** temperature, precipitation, seasonality, variability from paleoclimate
  reconstructions / transient simulations, downscaled to the grid. Holocene events
  (Younger Dryas, 8.2 ka, 4.2 ka events, Little Ice Age) come from data, not scripts.
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
  pH, coarse fragments; modern proxy, ADR-0045).
- **Coupling (later):** land use → regional climate/erosion; industrial emissions →
  global temperature via a reduced-complexity climate emulator (FaIR-like) post-1750.

## 4.2 Ecology & resources

- Biome productivity, wild plant/animal resources, megafauna remnants, fisheries.
  *Implemented (M1):* forager carrying capacity per cell from the climate pack (Miami
  model productivity), dry-land area and aquatic resources (rivers, lake shores,
  coasts), recomputed as the climate changes (`oikos-sim::environment`, ADR-0047).
- **Domesticable species pool** by region (wheat, barley, lentils, goats, sheep,
  cattle, pigs, rice, maize…), with suitability from agro-climatic models.
- Domestication as a co-evolutionary process (selection intensity × cultivation
  effort × time), not a switch.
  *Implemented (M1):* bands carry cultivation effort and the domestic share of their
  crop; effort moves toward a target nesting three competing drivers (population
  pressure, climate stress, wild-cereal abundance) as weights; cultivation selects for
  domestic traits against wild gene flow, slowly or quickly by the selection
  hypothesis; capacity blends foraging and farming by effort, so domestic crops feed
  more people and the transition can ratchet (`mechanisms::Cultivation`,
  `environment::cereal_suitability`, ADR-0049). A fourth, nested hypothesis scales the
  target by climate reliability (stable-climate hypothesis, ADR-0054; off by default).
  Fields wear the soil down and yields fall once it thins below a critical share (land
  degradation, ADR-0055; off by default).
  Wild-cereal suitability comes from temperature and precipitation; soils, irrigation
  and animals are not yet included.
- Minerals and energy: obsidian, flint, copper, tin, iron, salt, timber, later coal,
  oil, gas, uranium, renewables potential.

## 4.3 Demography & genetics

- Age-structured fertility/mortality conditional on nutrition, disease, workload,
  sedentism (Neolithic demographic transition), and later the modern demographic
  transition (education, child mortality, contraception, urbanization).
  *Implemented (M1):* logistic growth whose intrinsic rate moves with a band's
  cultivation effort from the mobile foragers' rate to the sedentary cultivators'
  (`mechanisms::Growth`, ADR-0053), the net effect of the Neolithic demographic
  transition; age structure is not yet modelled. Equal rates (the default) nest the
  no-transition hypothesis.
- Kinship and marriage rules (emergent norms: endogamy, polygyny, dowry/brideprice).
- Optional lightweight population genetics (lactase persistence, disease resistance,
  ancestry tracers) — chiefly as a **validation signal** against ancient DNA.

## 4.4 Cognition & behavior (psychology, neuroscience, behavioral economics)

Not brain simulation. Agents implement validated regularities:
- Bounded rationality, satisficing, heuristics; prospect-theory risk preferences;
  present bias; loss aversion.
- **Social learning** (conformity, prestige and success bias) — dual-inheritance
  theory (Boyd & Richerson).
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
- Markets: local price formation via decentralized bargaining/matching; routes with
  transport costs by terrain, technology (pack animals, ships, rail, containers).
- Specialization and division of labor tied to market size (Smith) and urbanization.
- Modern era: firms, labor markets, credit cycles, government budgets, trade,
  global value chains — informed by macro ABMs (e.g. EURACE, Axtell's firm model).

## 4.6 Institutions & power (political economy — first class)

- Groups form, merge and split by cooperative benefit vs. coordination cost.
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
- Nutrition, sanitation, medicine (germ theory, vaccines, antibiotics) as technologies.
- Modern health systems and disease burden calibrated to GBD data.

## 4.9 Conflict & migration

- Raiding, warfare (technology, logistics, terrain), conquest, refugee flows.
- Migration driven by expected utility differentials, networks, and push factors
  (climate shocks, famine, violence).
  *Implemented (M1):* forager dispersal: crowded cells bud off groups into less
  crowded neighbors with room, the wave of advance by which foragers fill the region
  (`mechanisms::Dispersal`, ADR-0048).

## 4.10 Modern-era additions (1800 →)

- Industrialization (energy, machines, factories), the demographic transition,
  urbanization, mass education, nation-states, colonialism and decolonization,
  globalization, digital networks and **AI/automation** as a technology with labor-
  market, productivity and power-concentration effects.

## 4.11 Welfare indicators (computed everywhere, all eras)

Health & longevity · material wellbeing (consumption, poverty, inequality) ·
capabilities & freedom (education, agency, violence, political freedom) ·
sustainability (resource stocks, emissions, ecological integrity). See doc 6.
