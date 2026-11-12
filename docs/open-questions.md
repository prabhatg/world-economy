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
33. What record sees village size and aggregation (ADR-0073; #34)? Farmers' villages split
    by scalar stress (`village_fission_size`, ADR-0070) and settled groups join the
    largest village of their cell (`aggregation_rate`, ADR-0071), but none of the five
    M1 records responds to either: across round 9's consistent worlds both spread over
    their priors as all runs do, so the size limit stays at its literature value (127,
    Alberti 2014) and the joining rate a placeholder. Survey site areas (settlement-size
    distributions by era, as in the Anatolian and Upper Mesopotamian surveys already
    used) or excavated house counts per phase could; an operator would compare them
    with the distribution of group sizes the model's settlements reach. The same holds
    for fertility and sedentary growth until the juvenile-index record (#33) joins.
    The survey site areas now join (ADR-0078): at the calibrated defaults settlements
    hold about 50 people (geometric mean), against 126-218 at the middle density, so
    the defaults fit only if Neolithic sites were as thinly lived in as Kuijt and
    Marciniak (2024) find at Çatalhöyük. Through time the record's villages grow in
    Upper Mesopotamia (126 to 179 people) while the model's stay level. Round 10's wave 4
    judges it: the record narrows no parameter on its own, and size follows how densely
    farming settles more than `village_fission_size`, which few settlements reach. The
    consistent world nearest the record's sizes (179 people) has groups that seldom
    disperse and often join, and fits the radiocarbon and onset records worse than the
    defaults. Still open: why villages do not grow through time, which towns of several
    groups (#39) or a density that changed with house forms could explain. The defaults
    of ADR-0081 (wave 6 run 216, `village_fission_size` 186) hold about 67 people.
    Why they are small (ADR-0082): farmers settle at about 0.38 of their effort
    cultivating, where a group's limit is about 92 people, and they reach it; a cell
    holds five or six such groups. Settlements of several groups (ADR-0082) bring the
    level into the record's range at the defaults (110-160 people from two groups'
    worth); they leave it level through time (open question 38).
34. How strongly do houses track wealth, and when did inequality rise (ADR-0074; #43)? The
    house-size operator takes floor area to grow as wealth to the power 0.7, a judgment
    from comparing house-size and wealth Ginis across two compilations; within-site data
    on house size and household wealth would set it. The five M1 sites' Ginis rise from
    the PPNA to the Late Chalcolithic, while the calibrated defaults' stay near 0.25-0.27
    throughout: people around every site plant from early on, so land is partly closed
    everywhere. With two to nineteen houses a site the record cannot yet tell these
    apart; round 10's wave 4, the first judged on the Ginis, keeps that picture (its
    consistent worlds' Ginis 0.25-0.26 at every site, the wealth parameters spread over
    their priors). More sites with measured houses (Kohler and Smith 2018 list others outside the
    supplement) would. The generation length (28 years, Fenner 2005) is to be checked
    against the paper.
35. How heavy was disease in Neolithic villages, and at what size did it persist
    (ADR-0075; #42)? The critical size (50 sedentary people), foragers' relative contact (0.4)
    and the mortality of the infected (0.005 a year) are judgments. The records bound the
    mortality from above only indirectly: at four times the default, domestication lags
    (a domestic barley share after 7000 BCE of 0.24, against the observed 0.80). The skeletal record would test it directly:
    Eshed et al. (2010) compare lesions of infection in 200 Natufian and 205 Neolithic
    skeletons of the southern Levant, which an operator now compares, as the Neolithic
    share of the lesioned, with the prevalence around the sampled sites (ADR-0086): it sees
    the rise, which leans toward foragers spreading less infection (`mobile_contact`
    below its 0.4), not the mortality. Behavioral responses
    (villages splitting or dispersing under disease, hygiene norms) are left out until a
    record calls for them.
36. How strongly did people copy farming for its payoff, and how far did they look
    (ADR-0076; #36)? Success bias at 1 brings domestication and cultivation forward of the
    record in the calibrated world, which was fitted without it, to the edge of the
    record (implausibility 2.86-2.98), so it is 0 by default for now; the bias trades off
    against `adoption_rate` and `innovation_share`, so the three are to be calibrated
    together (round 10). Round 10 (ADR-0077) did, and they do not: among its 54
    consistent worlds success bias spreads over its whole prior (u 0.11-0.92), traded
    against the adoption rate and innovation share, so it stays 0. The onset and rachis records may not separate a faster spread
    where farming pays from a faster spread everywhere; the spatial pattern of the onset
    (does cultivation start in the richest stands and spread outward?) could. Neighbor
    learning (0.1) changes the fit little (2.26-2.28 against 2.32-2.34): the M1 records
    do not yet see how far practice is learned. The onset contrasts (ADR-0080) test that
    pattern, and it does not separate them either: every consistent world fits the
    contrast, which follows how much people weigh the abundance of wild stands
    (`cultivation_abundance_weight`, Spearman 0.77 over wave 6) rather than whom they
    copy (success bias -0.18), and success bias still spans its prior (u 0.04-0.94).
    What could: the order in which neighboring sites took up farming (a front moving
    out from the richest stands under success bias, everywhere at once under
    adoption), which needs phases dated finely enough to order neighbors, or a record
    of who farmed within a site (house-level plant remains against house size). The
    defaults of ADR-0081 carry 0.23, the chosen consistent world's value, not a measurement.
37. Why does the model's periphery fill too early and its core too late (ADR-0079;
    #35)? Judged by subregion, worlds consistent with the earlier records date the first
    tenth of activity 900-2,200 years too early in Anatolia and 350-1,500 in the Zagros
    (10-90% of the worlds; z about -2.5 and -1.9) and the Levant's half 250-1,000 years
    too late. Candidates: a forager density that does not
    fall with the Younger Dryas cold on the plateaus; research that sought Neolithic tells
    in Anatolia more than Epipalaeolithic camps (intensity that changes with the period
    does not cancel within a block, unlike constant intensity); or core villages that
    grow too slowly (the village sizes of ADR-0078 point the same way, #39). Northeast
    Africa is judged since ADR-0095 (#44, open question 40).
38. What made Upper Mesopotamian villages grow from the Halaf to the Late Chalcolithic
    (126 to 179 people at the middle density) while Central Anatolia's stayed level
    (ADR-0078, ADR-0082)? Settlements of several groups can reach the record's level
    (two to two and a half groups' worth gives 113-147 people), but the model's stay level in both surveys: integration is a constant, and the
    inequality that hierarchy reads hardly changes once the land has filled (the
    settled people's variance of log wealth is 0.28 from 9000 to 3000 BCE at the
    defaults with towns, as they farm about 0.38 of their effort). At the calibrated
    integration (four groups' worth, ADR-0083) villages are about 100 people in every
    era of both surveys, since a cell's land holds one town and the small groups it
    sheds. A settlement is also capped by its own cell's land (about 400
    people at the defaults), so the largest sites, which drew on more land (Late
    Chalcolithic Tell Brak, over 100 ha), cannot form. Candidates: towns whose catchment
    reaches into neighboring cells, as central places with craft and exchange; rising
    yields (irrigation, the plough, secondary products) that let more people live off
    one cell; conflict that drew people together for defence (Kuijt 2000; Birch 2013);
    or house forms whose density changed with the period, so that areas overstate the
    growth.
39. Did Arabia's early Holocene humid phase matter for the region (ADR-0084)? Lakes in the
    Nafud and speleothems in southern Arabia record one, and CHELSA-TraCE21k's climate model
    probably misses it as it misses the Sahara's, but the humid-period pack covers Africa
    only, as the reconstructions it extrapolates are African. Northern Arabia is in the
    region, judged only region-wide.
40. Why is the Nile valley almost undated before 9,000 BCE, when the model keeps some
    4,300 foragers there from 12,000 BCE (ADR-0084)? Candidates: Early Holocene floods
    that made the valley less habitable during the Humid Period (Zaki et al. 2021,
    doi:10.1016/j.quascirev.2021.107200), sites buried under the aggrading floodplain and
    delta, which no survey sees, or both. And how did farming reach the Nile? Its
    floodplain can be farmed since ADR-0085, but the model's farmers stop at Sinai's
    170 km of desert: their daughters move only to neighboring cells, the Nile's are full
    of riverine foragers, and seed passed by learning stops where nobody farms. Herders
    who crossed with their flocks (#37), or migrants who travelled farther, are the
    candidates. Herders do not cross either, even since they judge land by what their
    herds would graze (#45): they reach the Negev (about a tenth of effort by 4,000
    BCE), but Sinai's median rain is about 45 mm and only a tenth of its cells reach
    the 150 mm where the model's pasture begins (`pasture_precip_min`, a judgment for
    the Fertile Crescent's margins), so it offers no grazing. Herders there lived on
    wadis, springs and the coast, at well under 150 mm. Since ADR-0092 wadis graze
    flocks in deserts, and need can drive herding through domestic stock: herders then
    cross the north coast (117-162 mm) to the Nile, but only with strong need and wadis
    at the top of their prior, from about 4,000 BCE, and the sheep-size record prefers
    need off. Is the crossing a matter of the herding parameters, which a round can vary
    together, or of something the model lacks (pack animals, water points, the coast's
    fishing)? And crossing does not move the subregion's dates: the Nile's foragers from
    12,000 BCE date it millennia early in every setting, so the first question decides
    whether the subregion can be judged. Since ADR-0093 most of that came instead from
    the Western Desert, which the model left empty: with the Humid Period's rain over
    Egypt fitted to the desert's own estimates, at four times the fit, it is settled, and
    the 25% and 50% dates match the record. Since ADR-0095 the Delta and Cairo block's
    2,200-3,500 foragers from 12,000 BCE are out of sight, on land drowned since and
    under the delta's silt before it aggraded (7,500-5,500 BCE), and the subregion is
    judged: its 10% point is about 700 years early at the defaults (implausibility about
    2), less at a stronger Humid Period. Open still: whether the Nile valley held fewer
    foragers during the Humid Period, when floods were high and the desert drew people
    away (Kuper and Kropelin 2006), which burial can hide but not tell apart. Since
    ADR-0098 the lakes a modern work filled (the Bitter Lakes, by the Suez Canal) are
    out of the hydrology pack; the Delta's lagoons stay, natural if Holocene, and their
    shores still give foragers the aquatic bonus from 12,000 BCE. Since ADR-0113 (#51)
    Egypt's first domestic animals are recorded (Linseele et al. 2014): every assemblage
    of the 6th and 5th millennia BCE holds some, and no world at the calibrated defaults
    brings any to Africa. Animals passed along social ties and grazing on floodplains
    do not either; with need at 0.8 as well, herds reach the Nile and the Fayum in the
    5th millennium, but the Levant's sheep turn domestic a millennium early. A reserve of
    animals kept wherever any grazing allows (ADR-0114) brings them there without that
    cost (0.79 of the 5th millennium's assemblages at 0.2), but the 6th millennium's
    desert assemblages still show them at 0.19 at most. Herders who find no room next
    door and travel on to the nearest land that can take them (ADR-0115), as the
    desert groups moved 150-200 km, bring them to Djara and Sodmein: at 200 km the 6th
    millennium's assemblages show them at 0.46-0.60 and the 5th's at 0.99, every world
    consistent; Farafra's Hidden Valley, deepest in the desert, stays without. Judged
    since ADR-0116, the record calibrates all four on (reserve 0.2, exchange 0.2,
    floodplain grazing 0.25, reach 250 km): the 6th millennium's assemblages show
    domestic animals at 0.60-0.67, the 5th's at 0.999, the reserve and the reach at the
    tops of their priors; beyond them the reach is flat and a larger reserve brings
    crops to the Fayum too early (ADR-0118). Hidden Valley's herds stay near 0: the
    herds' domestic share fades westward from the Nile through the populated desert,
    to 0.01 there, even once merging keeps it (ADR-0119), as herding follows the
    herd's domestic share and the desert grazes little. Their crops, judged since
    ADR-0117, fit at the defaults: none in the 6th millennium or the deserts, crops in
    the Nile valley and the Fayum in the 5th; without seed exchange the Fayum's
    neighbours farmed in the 6th millennium, but only before merging kept domestic
    shares (ADR-0121): now seed exchange brings them crops earlier, not later. Round 15 keeps the defaults
    (ADR-0120): in its worlds Egypt's crops reach the Nile by the 5th millennium only
    where barley is selected fast enough, which the Levant's early barley caps. Is the
    rest the desert foragers' own wide ties (#52), along which a few animals passed (Hidden
    Valley's hunters kept a few goats far from any herders' new home)? And the crops: the model's Mediterranean coast of Egypt domesticates its own
    wild barley from about 8,000 BCE, while Egypt's earliest crops (Merimde, the Fayum,
    the 5th millennium BCE) came from Southwest Asia with emmer, which never grew wild
    there. Wild barley is native to Egypt's coast (Euro+Med; Boulos's checklist), though
    GBIF's open records place it only near Mersa Matruh, so its range is not cut there;
    the coast's early fields lie on land since drowned or under the Delta's silt, which
    no record sees (#51).
41. Did the 8.2 ka event shape Southwest Asia's Neolithic, and how did people ride it out
    (ADR-0107)? The model takes its decadal anomalies from Greenland (NGRIP), which
    dates the event but gives one index for the whole region, with decadal noise mostly
    Greenland's own. The region's speleothems (Soreq, Jeita, Sofular, Katalekhor in
    SISALv3) do not give the Levant's and Anatolia's own decades instead: below a few
    centuries they agree neither with each other nor on the event, their dating
    uncertainty larger than the swings (ADR-0109). At literature sizes (-1 degC, -15%
    rain) the event moves Anatolia's site counts toward the record's fall in
    6500-6000 BCE but not its rebound after; with the stable-climate hypothesis' weight
    recalibrated to the real swings (7.2 to 2) the fit is as good as without the
    anomalies, so the record neither asks for the event nor rules it out. Flohr et al. (2016, doi:10.1016/j.quascirev.2015.06.022) find no
    region-wide abandonment, only local change: resilience the model's people may lack,
    since they meet a bad decade only by cultivating more under stress, moving and
    passing seed along ties, without storage, sharing or diversifying (Halstead and
    O'Shea 1989).
42. Where did wild emmer and wild einkorn grow in the early Holocene (ADR-0108)? The
    `wild_wheats` pack takes today's ranges from GBIF's occurrence records, which follow
    collecting missions and roads, miss what grazing and ploughing have cleared since,
    and know nothing of the Younger Dryas, when the stands likely retreated to refugia
    (Willcox 2005). A climate envelope fitted to the records and projected onto
    CHELSA-TraCE21k's centuries would give ranges that move with the climate, at the
    cost of over-predicting where dispersal, not climate, limits the wheats (wild emmer
    is absent from much land that suits it). Whether early cultivation beyond the ranges
    (Tell Aswad, Aşıklı Höyük) sped domestication by isolating sown stands from wild ones,
    as Willcox argues, is what the ranges let calibration weigh.
43. Why were the Jilat camps' cereals wild while the model reads them domestic (ADR-0110)?
    Jilat 7 (7775 BCE, 0 of 85 barley ears domestic) and Jilat 13 (6815 BCE, 1 of 23) were
    camps in the eastern Jordanian desert (Allaby et al. 2017's tables), while El Kowm 2, an
    oasis village of about the same age, grew wholly domestic barley and einkorn. In the
    model, seed exchange along ties (ADR-0102) gives the few groups sowing near Jilat
    their neighbours' domestic seed (0.65 and 0.83 at the defaults), and the wild ears
    foragers gather beside them (ADR-0103) are too few to outweigh it. Mobile people who
    harvested wild stands where they found them, without keeping plots, may not have
    taken up the seed of sedentary farmers at all; or the assemblage records one camp,
    not the 1,760 km2 the operator pools (a finer grid, F8, would narrow it). The model
    cannot yet tell the two places apart: the people near El Kowm and near Jilat put the
    same share of their effort into sowing (0.18 and 0.19 of people-years, seeds 1-3), so
    no rule keyed to sedentism, which the model measures by that effort, would separate
    them. What did was most likely El Kowm's water, which brings back springs (ADR-0110).
44. How did people answer harvest risk (ADR-0122)? Harvests now swing from year to year,
    more where rain is scarce, and the records are consistent only if foragers and
    herders feel about a fifth of a farmer's swings, as if they moved to where the rain
    fell. The answers the literature describes are storage, sharing, moving in bad years
    and spreading effort over crops and herds (Halstead and O'Shea 1989); which of them
    a record can tell apart is open. Storage is in (ADR-0123); sharing between groups and
    moving in bad years are not.

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
11. ~~Grid~~ — H3 (ADR-0009). Remaining: refinement policy. At resolution 6 (ADR-0111) M1
    costs 2.3 times as much but fit worse at its resolution-5 calibration: villages gathered
    and infection persisted only within a cell, so both shrank with the cell. Land and
    settlement are now counted per catchment, a block of cells about 8 km in radius
    (ADR-0112, #50), and resolution 6 fits nearly as well with the same parameters. Open:
    whether a finer grid earns its cost (where in a catchment villages stand, the land's
    detail), and the nested design.
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
