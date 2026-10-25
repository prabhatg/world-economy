# 12. Project identity & credit

## 12.1 Name (proposal — final choice is the founder's)

| Candidate | Meaning / fit | Risks |
|---|---|---|
| **ManyWorlds** *(recommended public name)* | Every volunteer's machine runs a different possible history; science is done over the ensemble of many worlds; the goal is a better one. Easy to say and spell; nods to the `@home` tradition as "ManyWorlds@home" for the volunteer network. | Association with the quantum "many-worlds" interpretation; generic phrase may be hard to trademark. |
| **Oikoumene** | Greek "the whole inhabited world"; shares the root *oikos* (household) with *economy* and *ecology*. Distinctive, scholarly. | Hard to spell/pronounce for a public brand. |
| **Commonweal** | "The common good" — states the mission directly. | An unrelated Scottish commons project uses the word. |
| **Hearth** | First settled homes; warmth, wellbeing, home. | Soft; less descriptive of simulation. |
| **Humanity@home** | Direct homage to SETI@home. | Least distinctive. |

Suggested structure:
- **ManyWorlds** — the nonprofit project, website and volunteer apps ("Run a world. Help find a better one.").
- **Oikos** — codename of the simulation engine (Rust crates `oikos-*`).

Initial web searches found no existing project under these names; a proper trademark,
domain and app-store name search is required before adoption.

## 12.2 Credit for everyone

The project only works because many kinds of people contribute. All of them get
visible, durable credit.

### Who is credited

| Contributor | How they are credited |
|---|---|
| **Compute volunteers** | Compute credit (normalized FLOP), per-world and per-experiment attribution, team credit; named (opt-in) in every result's credit manifest |
| **Explorers** | Credit for worlds run and shared; featured worlds |
| **Citizen scientists** | Credit per verified parameter/mechanism/record review, weighted by agreement with expert review |
| **Domain experts** | Reviewer credit on mechanisms and results; co-authorship where contribution meets authorship norms |
| **Developers, designers, translators, writers** | All Contributors-style recognition for code and non-code contributions |
| **Researchers whose work we use** | Every paper behind a mechanism or prior is cited in the knowledge base and automatically in any result that depends on it |
| **Dataset creators** | Datasets cited per their license terms in every dependent result |
| **Institutions, donors, funders, cloud/HPC providers** | Acknowledged in the credits ledger and publications |
| **Community moderators and organizers** | Recognized in the credits ledger |
| **AI assistants** | Acknowledged as tools (e.g. Claude, Fable) in the development credits, consistent with publication norms; tools are not authors |

### How it works

- **Credits ledger:** an open, append-only public dataset of contributions (pseudonymous
  by default; display names opt-in), exportable and versioned.
- **Provenance-driven attribution:** because every result records which engine
  version, mechanisms, parameters, datasets, papers and volunteer hosts produced it
  (docs 3, 5, 11), each published result ships with an automatically generated
  **credit manifest** naming all of them.
- **Academic credit:** contributor roles recorded with the CRediT taxonomy; papers list
  a consortium author ("the ManyWorlds Contributors") with the full contributor list
  attached, following precedents from Foldit and climateprediction.net.
- **Code and docs:** commits keep authorship and co-author trailers; an
  All Contributors table in `CREDITS.md` covers non-code roles.
- **No pay-to-rank:** credit reflects contribution, never donations.
