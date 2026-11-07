//! The calibration record as a run output (#46).
//!
//! Observation operators score a world by its activity per cell and century, sampled
//! every few years (a left Riemann sum of each cell's state): people-years; the same
//! weighted by cultivation effort, by effort times the crop's domestic share and by
//! effort times the settled people's wealth inequality; per obsidian source group,
//! people-years times its availability; settlement-years, settlement-years times their log
//! size and infected people-years; herders, their domestic herds, and the foragers hunting
//! where wild sheep and goats range. `oikos_cal.record.activity` computed these in Python
//! through the bindings; writing them here lets a calibration world run wherever the CLI
//! runs, with the operators scoring on the calibration host. The arithmetic follows the
//! Python record operation by operation, so the two are bit-identical.
//!
//! The record is a Zarr group `activity.zarr` of float64 arrays [century, cell], with
//! each cell's H3 index. Recording only reads the world, so it never changes a run's
//! state hash, and its bytes are identical for any partition count.
//!
//! A full M1 record is about 94 MB, too much for a volunteer to upload. Every operator
//! reads it as per-century sums over cell sets, so a work unit may name a zone file
//! (`oikos_cal.zones`, #47): the arrays are then [century, zone], each zone's sum of its
//! cells in ascending order, a weighted zone's cells each times its weight in that
//! century, exactly as `oikos_cal.zones.Zones.sums` adds them.

use crate::inputs::PackRef;
use crate::world::World;
use oikos_core::time::TICKS_PER_YEAR;
use oikos_io::{Sink, zarr};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivitySpec {
    /// Directory (relative to the host's output root) receiving `activity.zarr`.
    pub dir: String,
    /// Years between samples; must divide a century.
    #[serde(default = "default_every_years")]
    pub every_years: u64,
    /// A zone file, by id: record sums over its zones instead of every cell (#47).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zones: Option<PackRef>,
}

/// The zone file's format, as `oikos_cal.zones` writes it.
pub const ZONES_FORMAT: &str = "oikos-zones/1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ZoneFile {
    format: String,
    cells_hash: String,
    cells: usize,
    first_year: i64,
    centuries: usize,
    zones: Vec<Vec<usize>>,
    weighted: Vec<usize>,
    weights: WeightFile,
}

/// Weights of the cells listed (the others weigh 1), as rows of IEEE 754 bits per century.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WeightFile {
    cells: Vec<usize>,
    row: Vec<usize>,
    rows: Vec<Vec<u64>>,
}

/// Cell sets to sum the record over, some weighted per cell and century.
struct Zones {
    id: String,
    cells: Vec<Vec<usize>>,
    weighted: Vec<bool>,
    /// Per cell, its row of `rows`, if it has one.
    row: Vec<Option<usize>>,
    rows: Vec<Vec<f64>>,
}

impl Zones {
    fn load(
        r: &PackRef,
        world: &World,
        first_year: i64,
        centuries: usize,
        read: &Read,
    ) -> Result<Zones, String> {
        let bytes = read(&r.path)?;
        let id = blake3::hash(&bytes).to_hex().to_string();
        if id != r.id {
            return Err(format!("zones {}: the file's hash is {id}", r.id));
        }
        let f: ZoneFile = serde_json::from_slice(&bytes).map_err(|e| format!("zones {}: {e}", r.id))?;
        let n = world.grid.len();
        let h3: Vec<u64> = (0..n as u32).map(|c| world.grid.h3(c)).collect();
        let err = |m: String| Err(format!("zones {}: {m}", r.id));
        if f.format != ZONES_FORMAT {
            return err(format!("format {}, not {ZONES_FORMAT}", f.format));
        }
        if f.cells != n || f.cells_hash != oikos_io::pack::cells_hash(&h3) {
            return err("made for other cells than the world's".into());
        }
        if f.first_year != first_year || f.centuries != centuries {
            return err(format!(
                "{} centuries from {}, the record {centuries} from {first_year}",
                f.centuries, f.first_year
            ));
        }
        let w = &f.weights;
        if f.zones.iter().flatten().chain(&w.cells).any(|&c| c >= n) {
            return err(format!("cells outside the world's {n}"));
        }
        if f.weighted.iter().any(|&z| z >= f.zones.len()) || w.row.iter().any(|&i| i >= w.rows.len()) {
            return err("a weighted zone or weight row that does not exist".into());
        }
        if w.cells.len() != w.row.len() || w.rows.iter().any(|r| r.len() != centuries) {
            return err(format!("each weighted cell needs a row of {centuries} centuries"));
        }
        if f.zones.iter().any(|z| z.windows(2).any(|p| p[0] > p[1])) {
            return err("each zone's cells ascending".into());
        }
        if w.cells.windows(2).any(|p| p[0] >= p[1]) {
            return err("weighted cells ascending, each once".into());
        }
        let first_weighted = f.zones.len().saturating_sub(f.weighted.len());
        if f.weighted.iter().enumerate().any(|(i, &z)| z != first_weighted + i) {
            return err("the weighted zones are the last ones, in order".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        if !f.zones.iter().enumerate().all(|(z, cells)| seen.insert((z >= first_weighted, cells))) {
            return err("each zone once".into());
        }
        let mut row = vec![None; n];
        for (&c, &i) in w.cells.iter().zip(&w.row) {
            row[c] = Some(i);
        }
        let mut weighted = vec![false; f.zones.len()];
        f.weighted.iter().for_each(|&z| weighted[z] = true);
        let rows = w.rows.iter().map(|r| r.iter().map(|&b| f64::from_bits(b)).collect()).collect();
        Ok(Zones { id, cells: f.zones, weighted, row, rows })
    }

    /// Each zone's sum of `values` (one per cell) in century `t`.
    fn sums(&self, t: usize, values: &[f64]) -> Vec<f64> {
        let weight = |c: usize| self.row[c].map(|i| self.rows[i][t]);
        (self.cells.iter().zip(&self.weighted))
            .map(|(cells, &weighted)| {
                cells.iter().fold(0.0, |acc, &c| match (weighted, weight(c)) {
                    (true, Some(w)) => acc + values[c] * w,
                    _ => acc + values[c],
                })
            })
            .collect()
    }
}

type Read<'a> = dyn Fn(&str) -> Result<Vec<u8>, String> + 'a;

fn default_every_years() -> u64 {
    10
}

/// The record's arrays, in the order they are written; obsidian follows, per source.
pub const FIELDS: [(&str, &str); 10] = [
    ("people", "people-years"),
    ("cultivators", "people-years times cultivation effort"),
    ("domestic", "people-years times cultivation effort times the crop's domestic share"),
    ("settlements", "settlement-years, each group counted as far as it is settled"),
    ("inequality", "people-years times cultivation effort times the settled's variance of log wealth"),
    ("village_size", "settlement-years times the log of their people"),
    ("infected", "infected people-years"),
    ("herders", "people-years times herding effort"),
    ("herd_domestic", "people-years times herding effort times the herd's domestic share"),
    ("hunters", "people-years times foraging effort times wild sheep and goats' range"),
];

/// Cells per chunk: a century of one array is a few chunks.
const CELL_CHUNK: u64 = 8192;

pub struct ActivityRecorder {
    spec: ActivitySpec,
    first_year: i64,
    end_year: i64,
    /// The century being summed: one row per field, then per obsidian source.
    rows: Vec<Vec<f64>>,
    series: Vec<zarr::TimeSeries<f64>>,
    files: Vec<String>,
    zones: Option<Zones>,
    /// Centuries written so far.
    century: usize,
}

impl ActivityRecorder {
    /// Starts recording `years` from the world's current year, taking the first sample.
    pub fn start(world: &World, spec: ActivitySpec, years: u64, read: &Read) -> Result<Self, String> {
        let first_year = world.tick.calendar_year();
        if world.tick.0 % TICKS_PER_YEAR != 0 || first_year % 100 != 0 || years % 100 != 0 || years == 0 {
            return Err(format!(
                "activity: the record spans whole centuries, from {first_year} for {years} years"
            ));
        }
        if spec.every_years == 0 || 100 % spec.every_years != 0 {
            return Err(format!("activity: every_years must divide a century, not {}", spec.every_years));
        }
        if (spec.every_years * TICKS_PER_YEAR) % world.base_period_ticks() != 0 {
            return Err(format!("activity: every_years {} is not a whole number of steps", spec.every_years));
        }
        let zones = (spec.zones.as_ref())
            .map(|r| Zones::load(r, world, first_year, (years / 100) as usize, read))
            .transpose()?;
        let cells = world.grid.len() as u64;
        let (width, dim) = match &zones {
            Some(z) => (z.cells.len() as u64, "zone"),
            None => (cells, "cell"),
        };
        let sources = world.obsidian_sources();
        let names =
            FIELDS.iter().map(|(n, _)| n.to_string()).chain(sources.iter().map(|s| format!("obsidian/{s}")));
        let series = names
            .zip(FIELDS.iter().map(|(_, d)| d.to_string()).chain(
                sources.iter().map(|s| format!("people-years times the availability of obsidian from {s}")),
            ))
            .map(|(name, description)| {
                zarr::TimeSeries::new(
                    &format!("{}/activity.zarr/{name}", spec.dir),
                    width,
                    1,
                    CELL_CHUNK,
                    &["century", dim],
                    json!({ "description": description }),
                )
            })
            .collect::<Vec<_>>();
        let mut r = ActivityRecorder {
            rows: vec![vec![0.0; cells as usize]; series.len()],
            spec,
            first_year,
            end_year: first_year + years as i64,
            series,
            files: Vec::new(),
            zones,
            century: 0,
        };
        r.sample(world);
        Ok(r)
    }

    /// After every step: completes a century when one ends, and samples on schedule.
    pub fn observe(&mut self, world: &World, out: &mut Sink) -> Result<(), String> {
        if world.tick.0 % (self.spec.every_years * TICKS_PER_YEAR) != 0 {
            return Ok(());
        }
        let year = world.tick.calendar_year();
        if (year - self.first_year) % 100 == 0 {
            self.flush(out)?;
        }
        if year < self.end_year {
            self.sample(world);
        }
        Ok(())
    }

    fn flush(&mut self, out: &mut Sink) -> Result<(), String> {
        let files = &mut self.files;
        let mut sink = |p: &str, b: &[u8]| {
            files.push(p.to_string());
            out(p, b)
        };
        for (s, row) in self.series.iter_mut().zip(self.rows.iter_mut()) {
            match &self.zones {
                Some(z) => s.push(&z.sums(self.century, row), &mut sink)?,
                None => s.push(row, &mut sink)?,
            }
            row.iter_mut().for_each(|v| *v = 0.0);
        }
        self.century += 1;
        Ok(())
    }

    /// Adds one sample, weighted by the years it stands for, as `record.activity` does.
    fn sample(&mut self, world: &World) {
        let n = self.spec.every_years as f64;
        let (cultivation, domestication, inequality) =
            (world.cultivation(), world.domestication(), world.inequality());
        let (settlements, village_sizes, infected) =
            (world.settlements(), world.village_sizes(), world.infected());
        let (herding, herd_domestication) = (world.herding(), world.herd_domestication());
        let range = &world.farmland.range;
        let obsidian: Vec<Vec<f64>> =
            (0..world.obsidian_sources().len()).map(|i| world.obsidian(i)).collect();
        let [
            people,
            cultivators,
            domestic,
            settled,
            unequal,
            village,
            infections,
            herders,
            herd_domestic,
            hunters,
        ] = &mut self.rows[..FIELDS.len()]
        else {
            unreachable!("one row per field")
        };
        for (c, &pop) in world.population.iter().enumerate() {
            let np = n * pop as f64;
            people[c] += np;
            let sown = np * cultivation[c];
            cultivators[c] += sown;
            domestic[c] += sown * domestication[c];
            unequal[c] += sown * inequality[c];
            settled[c] += n * settlements[c];
            village[c] += n * village_sizes[c];
            infections[c] += n * infected[c];
            let herded = np * herding[c];
            herders[c] += herded;
            herd_domestic[c] += herded * herd_domestication[c];
            let foraging = (1.0 - cultivation[c] - herding[c]).max(0.0);
            hunters[c] += np * foraging * range.get(c).copied().unwrap_or(0.0);
        }
        for (row, held) in self.rows[FIELDS.len()..].iter_mut().zip(&obsidian) {
            for (c, &pop) in world.population.iter().enumerate() {
                row[c] += n * pop as f64 * held[c];
            }
        }
    }

    /// Writes the arrays' metadata, the cells' H3 indices and the group; returns the
    /// files written.
    pub fn finish(mut self, world: &World, out: &mut Sink) -> Result<Vec<String>, String> {
        let files = &mut self.files;
        let mut sink = |p: &str, b: &[u8]| {
            files.push(p.to_string());
            out(p, b)
        };
        for s in self.series {
            s.finish(&mut sink)?;
        }
        let root = format!("{}/activity.zarr", self.spec.dir);
        if self.zones.is_none() {
            let h3: Vec<u64> = (0..world.grid.len() as u32).map(|c| world.grid.h3(c)).collect();
            let attrs = json!({ "description": "H3 index of each cell" });
            zarr::write_1d(&format!("{root}/cell_h3"), &h3, CELL_CHUNK, "cell", attrs, &mut sink)?;
        }
        if !world.obsidian_sources().is_empty() {
            zarr::write_group(&format!("{root}/obsidian"), json!({}), &mut sink)?;
        }
        let mut attrs = json!({
            "engine": crate::ENGINE,
            "first_year": self.first_year,
            "end_year": self.end_year,
            "every_years": self.spec.every_years,
            "obsidian_sources": world.obsidian_sources(),
            "description": "Activity per century and cell, sampled every every_years years (#46)",
        });
        if let Some(z) = &self.zones {
            attrs["zones"] = json!(z.id);
            attrs["description"] =
                json!("Activity per century, summed over each zone of the zone file (#47)");
        }
        zarr::write_group(&root, attrs, &mut sink)?;
        Ok(self.files)
    }
}
