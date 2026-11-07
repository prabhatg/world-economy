//! Super-individual agents in structure-of-arrays layout.
//!
//! Each agent represents `weight` people (Scheffer et al. 1995). Splitting and merging
//! change resolution while conserving people exactly. Agents also carry their practice
//! ([`Traits`]): split-off groups inherit it and merged bands take the people-weighted
//! mean, so practice travels with people (demic diffusion). The wealth inequality among
//! a group's households (ADR-0074) travels and pools the same way.
//!
//! Each group also lives in a settlement (ADR-0082), the site a survey finds: by itself
//! ([`ALONE`]), or with the groups of its cell that share its settlement label.

use oikos_core::EntityId;
use std::ops::Range;

/// Fixed-point one for shares in [0, 1] (parts per billion): exact integers, so merging
/// and checkpoints are the same on every host.
pub const SHARE_ONE: u32 = 1_000_000_000;

/// The settlement label of a group that lives by itself, a settlement of one group.
pub const ALONE: u32 = 0;

/// Index of emmer in [`Traits::wheat_domestication`].
pub const EMMER: usize = 0;
/// Index of einkorn in [`Traits::wheat_domestication`].
pub const EINKORN: usize = 1;

/// A band's practice (ADR-0049), as shares of [`SHARE_ONE`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traits {
    /// Share of subsistence effort spent cultivating.
    pub cultivation: u32,
    /// Share of domestic-type plants in the crop the band sows.
    pub domestication: u32,
    /// Variance of the logarithm of household wealth among the group's households
    /// (ADR-0074), in units of [`SHARE_ONE`] (so up to about 4.3, a Gini of 0.86).
    pub inequality: u32,
    /// Share of subsistence effort spent managing sheep and goats (#37); with
    /// `cultivation`, at most one.
    pub herding: u32,
    /// Share of domestic animals in the herd the band manages (#37).
    pub herd_domestication: u32,
    /// Shares of domestic-type plants in the emmer and the einkorn ([`EMMER`], [`EINKORN`])
    /// the band sows with its crop (#38); the crop's own share is barley's, which stands
    /// for the package in carrying capacity. One column, as they are copied together.
    pub wheat_domestication: [u32; 2],
}

impl Traits {
    /// Foragers: no cultivation, wild crops.
    pub const FORAGER: Traits = Traits {
        cultivation: 0,
        domestication: 0,
        inequality: 0,
        herding: 0,
        herd_domestication: 0,
        wheat_domestication: [0; 2],
    };

    pub fn share(x: u32) -> f64 {
        x as f64 / SHARE_ONE as f64
    }

    /// Per person, in [`SHARE_ONE`] units: cultivation effort, effort times the crop's
    /// domestic share, herding effort and effort times the herd's domestic share, the
    /// products truncated; a cell's farming sums are these times each group's people.
    pub fn practice(&self) -> [u64; 4] {
        let (one, e, h) = (SHARE_ONE as u64, self.cultivation as u64, self.herding as u64);
        [e, e * self.domestication as u64 / one, h, h * self.herd_domestication as u64 / one]
    }

    /// The nearest fixed-point share to `x`, clamped to [0, 1].
    pub fn from_share(x: f64) -> u32 {
        (oikos_math::finite(x).clamp(0.0, 1.0) * SHARE_ONE as f64).round() as u32
    }

    /// The fixed-point share for `x`, updated from `old`: rounded in the direction of the
    /// change (down when falling, up when rising), so small steady changes never stall
    /// at a rounding fixed point and decay reaches zero.
    pub fn toward(old: u32, x: f64) -> u32 {
        let v = oikos_math::finite(x).clamp(0.0, 1.0) * SHARE_ONE as f64;
        (if v < old as f64 { v.floor() } else { v.ceil() }) as u32
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Agents {
    pub id: Vec<EntityId>,
    pub cell: Vec<u32>,
    pub weight: Vec<u64>,
    /// [`Traits::cultivation`] per agent.
    pub cultivation: Vec<u32>,
    /// [`Traits::domestication`] per agent.
    pub domestication: Vec<u32>,
    /// [`Traits::inequality`] per agent.
    pub inequality: Vec<u32>,
    /// [`Traits::herding`] per agent.
    pub herding: Vec<u32>,
    /// [`Traits::herd_domestication`] per agent.
    pub herd_domestication: Vec<u32>,
    /// [`Traits::wheat_domestication`] per agent.
    pub wheat_domestication: Vec<[u32; 2]>,
    /// The settlement each group lives in: [`ALONE`], or a label it shares with the other
    /// groups of its settlement, unique within its cell (drawn from the cell's birth
    /// counter). Labels never span cells.
    pub settlement: Vec<u32>,
}

/// One settlement of a cell: its groups with people, and their people-weighted practice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settlement {
    /// [`ALONE`] for a lone group.
    pub label: u32,
    /// Index of its first group (canonical order).
    pub first: usize,
    pub groups: u32,
    pub people: u64,
    cultivation: u128,
    inequality: u128,
}

impl Settlement {
    fn of(label: u32, first: usize) -> Settlement {
        Settlement { label, first, groups: 0, people: 0, cultivation: 0, inequality: 0 }
    }

    /// Counts a group of `w` people with practice `t` in (one spawned this step, say).
    pub fn add_group(&mut self, w: u64, t: Traits) {
        self.add(w, t.cultivation, t.inequality);
    }

    fn add(&mut self, w: u64, cultivation: u32, inequality: u32) {
        if w > 0 {
            self.groups += 1;
            self.people += w;
            self.cultivation += w as u128 * cultivation as u128;
            self.inequality += w as u128 * inequality as u128;
        }
    }

    fn mean(sum: u128, people: u64) -> u32 {
        sum.checked_div(people as u128).unwrap_or(0) as u32
    }

    /// People-weighted mean cultivation effort of its groups (rounded down; a lone
    /// group's own).
    pub fn cultivation(&self) -> u32 {
        Settlement::mean(self.cultivation, self.people)
    }

    /// People-weighted mean of its groups' inequality (within groups: the model keeps no
    /// wealth differences between them).
    pub fn inequality(&self) -> u32 {
        Settlement::mean(self.inequality, self.people)
    }

    /// Mean cultivation and inequality if a group of `w` people with `t` joined it.
    pub fn joined(&self, w: u64, t: Traits) -> (u32, u32) {
        let mut s = *self;
        s.add(w, t.cultivation, t.inequality);
        (s.cultivation(), s.inequality())
    }
}

/// People-weighted trait sums of the small bands merging into one absorber. In u64:
/// overflow needs some 18 billion people in small bands of one cell, and u128 is slow
/// on wasm.
#[derive(Default)]
struct Pool {
    weight: u64,
    cultivation: u64,
    domestication: u64,
    /// In u128: inequality reaches u32::MAX, four times the shares' one.
    inequality: u128,
    herding: u64,
    herd_domestication: u64,
    wheat_domestication: [u64; 2],
}

impl Pool {
    fn add(&mut self, w: u64, t: Traits) {
        self.weight += w;
        self.cultivation += w * t.cultivation as u64;
        self.domestication += w * t.domestication as u64;
        self.inequality += w as u128 * t.inequality as u128;
        self.herding += w * t.herding as u64;
        self.herd_domestication += w * t.herd_domestication as u64;
        for (sum, &d) in self.wheat_domestication.iter_mut().zip(&t.wheat_domestication) {
            *sum += w * d as u64;
        }
    }

    /// People-weighted mean traits (rounded down).
    fn mean(&self) -> Traits {
        if self.cultivation == 0
            && self.domestication == 0
            && self.inequality == 0
            && self.herding == 0
            && self.herd_domestication == 0
            && self.wheat_domestication == [0; 2]
        {
            return Traits::FORAGER;
        }
        let w = self.weight.max(1);
        Traits {
            cultivation: (self.cultivation / w) as u32,
            domestication: (self.domestication / w) as u32,
            // u128 division is slow on wasm; the sum fits u64 below 4.3 billion people.
            inequality: match u64::try_from(self.inequality) {
                Ok(sum) => (sum / w) as u32,
                Err(_) => (self.inequality / w as u128) as u32,
            },
            herding: (self.herding / w) as u32,
            herd_domestication: (self.herd_domestication / w) as u32,
            wheat_domestication: self.wheat_domestication.map(|d| (d / w) as u32),
        }
    }
}

impl Agents {
    pub fn len(&self) -> usize {
        self.id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    /// Adds a band of foragers.
    pub fn push(&mut self, id: EntityId, cell: u32, weight: u64) {
        self.push_with(id, cell, weight, Traits::FORAGER);
    }

    /// Adds a group that lives by itself.
    pub fn push_with(&mut self, id: EntityId, cell: u32, weight: u64, traits: Traits) {
        self.push_in(id, cell, weight, traits, ALONE);
    }

    /// Adds a group living in the settlement labeled `settlement` of its cell.
    pub fn push_in(&mut self, id: EntityId, cell: u32, weight: u64, traits: Traits, settlement: u32) {
        self.id.push(id);
        self.cell.push(cell);
        self.weight.push(weight);
        self.cultivation.push(traits.cultivation);
        self.domestication.push(traits.domestication);
        self.inequality.push(traits.inequality);
        self.herding.push(traits.herding);
        self.herd_domestication.push(traits.herd_domestication);
        self.wheat_domestication.push(traits.wheat_domestication);
        self.settlement.push(settlement);
    }

    pub fn traits(&self, i: usize) -> Traits {
        Traits {
            cultivation: self.cultivation[i],
            domestication: self.domestication[i],
            inequality: self.inequality[i],
            herding: self.herding[i],
            herd_domestication: self.herd_domestication[i],
            wheat_domestication: self.wheat_domestication[i],
        }
    }

    pub fn set_traits(&mut self, i: usize, t: Traits) {
        self.cultivation[i] = t.cultivation;
        self.domestication[i] = t.domestication;
        self.inequality[i] = t.inequality;
        self.herding[i] = t.herding;
        self.herd_domestication[i] = t.herd_domestication;
        self.wheat_domestication[i] = t.wheat_domestication;
    }

    pub fn append(&mut self, other: &mut Agents) {
        self.id.append(&mut other.id);
        self.cell.append(&mut other.cell);
        self.weight.append(&mut other.weight);
        self.cultivation.append(&mut other.cultivation);
        self.domestication.append(&mut other.domestication);
        self.inequality.append(&mut other.inequality);
        self.herding.append(&mut other.herding);
        self.herd_domestication.append(&mut other.herd_domestication);
        self.wheat_domestication.append(&mut other.wheat_domestication);
        self.settlement.append(&mut other.settlement);
    }

    /// Appends copies of `other`'s agents at indices `span`.
    pub fn extend_from(&mut self, other: &Agents, span: Range<usize>) {
        self.id.extend_from_slice(&other.id[span.clone()]);
        self.cell.extend_from_slice(&other.cell[span.clone()]);
        self.weight.extend_from_slice(&other.weight[span.clone()]);
        self.cultivation.extend_from_slice(&other.cultivation[span.clone()]);
        self.domestication.extend_from_slice(&other.domestication[span.clone()]);
        self.inequality.extend_from_slice(&other.inequality[span.clone()]);
        self.herding.extend_from_slice(&other.herding[span.clone()]);
        self.herd_domestication.extend_from_slice(&other.herd_domestication[span.clone()]);
        self.wheat_domestication.extend_from_slice(&other.wheat_domestication[span.clone()]);
        self.settlement.extend_from_slice(&other.settlement[span]);
    }

    pub fn total_weight(&self) -> u64 {
        self.weight.iter().sum()
    }

    /// Canonical order: by cell, then id. Partition-independent by construction.
    pub fn sort_canonical(&mut self) {
        let mut order: Vec<usize> = (0..self.len()).collect();
        order.sort_unstable_by_key(|&i| (self.cell[i], self.id[i]));
        self.id = order.iter().map(|&i| self.id[i]).collect();
        self.cell = order.iter().map(|&i| self.cell[i]).collect();
        self.weight = order.iter().map(|&i| self.weight[i]).collect();
        self.cultivation = order.iter().map(|&i| self.cultivation[i]).collect();
        self.domestication = order.iter().map(|&i| self.domestication[i]).collect();
        self.inequality = order.iter().map(|&i| self.inequality[i]).collect();
        self.herding = order.iter().map(|&i| self.herding[i]).collect();
        self.herd_domestication = order.iter().map(|&i| self.herd_domestication[i]).collect();
        self.wheat_domestication = order.iter().map(|&i| self.wheat_domestication[i]).collect();
        self.settlement = order.iter().map(|&i| self.settlement[i]).collect();
    }

    /// Range of agents in `cells` (requires canonical order).
    pub fn cell_range(&self, cells: std::ops::Range<u32>) -> std::ops::Range<usize> {
        let a = self.cell.partition_point(|&c| c < cells.start);
        let b = self.cell.partition_point(|&c| c < cells.end);
        a..b
    }

    /// Removes zero-weight agents and merges every agent below `threshold` in a cell
    /// into the lowest-id small agent of that cell, which takes their people-weighted
    /// mean traits. Requires canonical order and keeps it.
    /// The reference for the fused [`Agents::merge_small_into`] used by the engine.
    pub fn merge_small(&mut self, threshold: u64) {
        let mut out = Agents::default();
        let mut i = 0;
        while i < self.len() {
            let cell = self.cell[i];
            let end = i + self.cell[i..].iter().take_while(|&&c| c == cell).count();
            let small: Vec<usize> =
                (i..end).filter(|&j| self.weight[j] > 0 && self.weight[j] < threshold).collect();
            for j in i..end {
                let w = self.weight[j];
                if w >= threshold.max(1) {
                    out.push_in(self.id[j], cell, w, self.traits(j), self.settlement[j]);
                } else if small.first() == Some(&j) {
                    let people: u64 = small.iter().map(|&k| self.weight[k]).sum();
                    let mean = |x: &[u32]| {
                        let s: u128 = small.iter().map(|&k| self.weight[k] as u128 * x[k] as u128).sum();
                        (s / people as u128) as u32
                    };
                    let t = Traits {
                        cultivation: mean(&self.cultivation),
                        domestication: mean(&self.domestication),
                        inequality: mean(&self.inequality),
                        herding: mean(&self.herding),
                        herd_domestication: mean(&self.herd_domestication),
                        wheat_domestication: [EMMER, EINKORN].map(|c| {
                            let s: u128 = small
                                .iter()
                                .map(|&k| self.weight[k] as u128 * self.wheat_domestication[k][c] as u128)
                                .sum();
                            (s / people as u128) as u32
                        }),
                    };
                    out.push_in(self.id[j], cell, people, t, self.settlement[j]);
                }
            }
            i = end;
        }
        *self = out;
    }

    pub fn clear(&mut self) {
        self.id.clear();
        self.cell.clear();
        self.weight.clear();
        self.cultivation.clear();
        self.domestication.clear();
        self.inequality.clear();
        self.herding.clear();
        self.herd_domestication.clear();
        self.wheat_domestication.clear();
        self.settlement.clear();
    }

    /// Whether every group of `groups` lives by itself, as before ADR-0082.
    pub fn alone(&self, groups: Range<usize>) -> bool {
        self.settlement[groups].iter().all(|&s| s == ALONE)
    }

    /// The settlement group `g` lives in, among the groups of `cell` (one cell's range).
    pub fn settlement_of(&self, cell: Range<usize>, g: usize) -> Settlement {
        let label = self.settlement[g];
        if label == ALONE {
            let mut s = Settlement::of(ALONE, g);
            s.add(self.weight[g], self.cultivation[g], self.inequality[g]);
            return s;
        }
        let first = cell.start + self.settlement[cell.clone()].iter().position(|&l| l == label).unwrap();
        let mut s = Settlement::of(label, first);
        for i in first..cell.end {
            if self.settlement[i] == label {
                s.add(self.weight[i], self.cultivation[i], self.inequality[i]);
            }
        }
        s
    }

    /// Whether group `g` lives in settlement `s`.
    pub fn lives_in(&self, g: usize, s: &Settlement) -> bool {
        if s.label == ALONE { g == s.first } else { self.settlement[g] == s.label }
    }

    /// Calls `f` with each settlement of `cell` (one cell's range) holding people, in order
    /// of their first group. Quadratic in the groups of a cell's settlements, which are few.
    pub fn for_each_settlement(&self, cell: Range<usize>, mut f: impl FnMut(&Settlement)) {
        for i in cell.clone() {
            let label = self.settlement[i];
            if label != ALONE && self.settlement[cell.start..i].contains(&label) {
                continue;
            }
            let s = self.settlement_of(i..cell.end, i);
            if s.people > 0 {
                f(&s);
            }
        }
    }

    /// Sorts into canonical (cell, id) order. All cells must lie in `cells`.
    ///
    /// Stable counting sort by cell (O(n + cells)), then insertion sort by id inside
    /// each cell. Each step's input is canonical plus a few appended agents, so the
    /// insertion sorts are near O(n). `scratch` and `counts` are reusable buffers.
    pub fn sort_in_cells(&mut self, cells: Range<u32>, scratch: &mut Agents, counts: &mut Vec<u32>) {
        let m = (cells.end - cells.start) as usize;
        let n = self.len();
        counts.clear();
        counts.resize(m + 1, 0);
        for &c in &self.cell {
            counts[(c - cells.start) as usize + 1] += 1;
        }
        for k in 0..m {
            counts[k + 1] += counts[k];
        }

        scratch.id.clear();
        scratch.id.resize(n, EntityId(0));
        scratch.cell.clear();
        scratch.cell.resize(n, 0);
        scratch.weight.clear();
        scratch.weight.resize(n, 0);
        scratch.cultivation.clear();
        scratch.cultivation.resize(n, 0);
        scratch.domestication.clear();
        scratch.domestication.resize(n, 0);
        scratch.inequality.clear();
        scratch.inequality.resize(n, 0);
        scratch.herding.clear();
        scratch.herding.resize(n, 0);
        scratch.herd_domestication.clear();
        scratch.wheat_domestication.clear();
        scratch.herd_domestication.resize(n, 0);
        scratch.wheat_domestication.resize(n, [0; 2]);
        scratch.settlement.clear();
        scratch.settlement.resize(n, ALONE);
        // Scatter; afterwards counts[k] is the end of cell k's group.
        for i in 0..n {
            let k = (self.cell[i] - cells.start) as usize;
            let dst = counts[k] as usize;
            counts[k] += 1;
            scratch.id[dst] = self.id[i];
            scratch.cell[dst] = self.cell[i];
            scratch.weight[dst] = self.weight[i];
            scratch.cultivation[dst] = self.cultivation[i];
            scratch.domestication[dst] = self.domestication[i];
            scratch.inequality[dst] = self.inequality[i];
            scratch.herding[dst] = self.herding[i];
            scratch.herd_domestication[dst] = self.herd_domestication[i];
            scratch.wheat_domestication[dst] = self.wheat_domestication[i];
            scratch.settlement[dst] = self.settlement[i];
        }

        let mut start = 0;
        for &end in &counts[..m] {
            let end = end as usize;
            for j in start + 1..end {
                let id = scratch.id[j];
                // Most agents are already in place; only load the rest of one that moves.
                if scratch.id[j - 1] <= id {
                    continue;
                }
                let (w, t, s) = (scratch.weight[j], scratch.traits(j), scratch.settlement[j]);
                let mut i = j;
                while i > start && scratch.id[i - 1] > id {
                    scratch.id[i] = scratch.id[i - 1];
                    scratch.weight[i] = scratch.weight[i - 1];
                    scratch.cultivation[i] = scratch.cultivation[i - 1];
                    scratch.domestication[i] = scratch.domestication[i - 1];
                    scratch.inequality[i] = scratch.inequality[i - 1];
                    scratch.herding[i] = scratch.herding[i - 1];
                    scratch.herd_domestication[i] = scratch.herd_domestication[i - 1];
                    scratch.wheat_domestication[i] = scratch.wheat_domestication[i - 1];
                    scratch.settlement[i] = scratch.settlement[i - 1];
                    i -= 1;
                }
                scratch.id[i] = id;
                scratch.weight[i] = w;
                scratch.set_traits(i, t);
                scratch.settlement[i] = s;
            }
            start = end;
        }
        std::mem::swap(self, scratch);
    }

    /// [`Agents::merge_small`] into `out`, also writing per-cell population to `pop`
    /// (indexed by `cell - cell_base`, zeroed first; empty to skip). Requires canonical
    /// order.
    pub fn merge_small_into(&self, threshold: u64, out: &mut Agents, cell_base: u32, pop: &mut [u64]) {
        out.clear();
        pop.iter_mut().for_each(|p| *p = 0);
        let keep = threshold.max(1);
        let mut i = 0;
        while i < self.len() {
            let cell = self.cell[i];
            let mut absorber: Option<usize> = None;
            let mut pool = Pool::default();
            let mut total = 0;
            // Agents kept as they are are copied in runs (one slice copy per column).
            let mut run = i;
            while i < self.len() && self.cell[i] == cell {
                let w = self.weight[i];
                total += w;
                if w >= keep {
                    i += 1;
                    continue;
                }
                out.extend_from(self, run..i);
                if w > 0 {
                    if absorber.is_none() {
                        absorber = Some(out.len());
                        out.push_in(self.id[i], cell, 0, Traits::FORAGER, self.settlement[i]);
                    }
                    pool.add(w, self.traits(i));
                }
                i += 1;
                run = i;
            }
            out.extend_from(self, run..i);
            if let Some(a) = absorber {
                out.weight[a] = pool.weight;
                out.set_traits(a, pool.mean());
            }
            if !pop.is_empty() {
                pop[(cell - cell_base) as usize] = total;
            }
        }
    }

    /// Moves up to `take` people out of `weight`; returns how many moved (the weight of
    /// the new agent).
    pub fn split_weight(weight: &mut u64, take: u64) -> u64 {
        let take = take.min(*weight);
        *weight -= take;
        take
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_practice_is_its_effort_and_domestic_shares_truncated_as_cells_sum_them() {
        let t = Traits {
            cultivation: 499_999_999,
            domestication: 500_000_001,
            herding: 300_000_001,
            herd_domestication: 333_333_333,
            ..Traits::FORAGER
        };
        // 499,999,999 x 0.500000001 = 249,999,999.999999999: truncated, as a cell's sum is.
        assert_eq!(t.practice(), [499_999_999, 249_999_999, 300_000_001, 100_000_000]);
        assert_eq!(Traits::FORAGER.practice(), [0; 4]);
    }

    fn sample() -> Agents {
        let mut a = Agents::default();
        a.push(EntityId::new(1, 3), 1, 4);
        a.push(EntityId::new(0, 1), 0, 50);
        a.push(EntityId::new(1, 1), 1, 5);
        a.push(EntityId::new(0, 2), 0, 0);
        a.push(EntityId::new(1, 2), 1, 70);
        a
    }

    #[test]
    fn merge_conserves_people() {
        let mut a = sample();
        let before = a.total_weight();
        a.sort_canonical();
        a.merge_small(10);
        assert_eq!(a.total_weight(), before);
        assert_eq!(a.len(), 3);
        // The two small agents in cell 1 merged into the lower id.
        let i = a.id.iter().position(|&id| id == EntityId::new(1, 1)).unwrap();
        assert_eq!(a.weight[i], 9);
    }

    #[test]
    fn shares_change_by_at_least_one_unit_and_decay_to_zero() {
        let mut e = 25;
        for _ in 0..1000 {
            e = Traits::toward(e, Traits::share(e) * 0.98);
        }
        assert_eq!(e, 0, "nearest rounding would stall at 25 parts per billion");
        assert_eq!(Traits::toward(10, Traits::share(10) * 1.01), 11);
        assert_eq!(Traits::toward(SHARE_ONE, 1.5), SHARE_ONE);
        assert_eq!(Traits::toward(7, Traits::share(7)), 7);
    }

    #[test]
    fn merged_bands_take_the_people_weighted_mean_practice() {
        let mut a = Agents::default();
        let t = |e: f64, d: f64, v: f64| Traits {
            cultivation: Traits::from_share(e),
            domestication: Traits::from_share(d),
            inequality: Traits::from_share(v),
            ..Traits::FORAGER
        };
        a.push_with(EntityId::new(0, 1), 0, 3, t(1.0, 0.5, 0.8));
        a.push_with(EntityId::new(0, 2), 0, 1, t(0.0, 0.1, 0.2));
        a.push_with(EntityId::new(0, 3), 0, 50, t(0.2, 0.2, 0.3));
        let mut out = Agents::default();
        a.merge_small_into(10, &mut out, 0, &mut [0]);
        assert_eq!(out.len(), 2);
        assert_eq!(out.traits(0), t(0.75, 0.4, 0.65), "(3 x 1 + 1 x 0) / 4, (3 x 0.5 + 1 x 0.1) / 4, ...");
        assert_eq!(out.traits(1), t(0.2, 0.2, 0.3), "large bands keep their own");
    }

    #[test]
    fn merged_bands_pool_their_herds_too() {
        // Herding effort and the herd's domestic share (#37) are people-weighted like the
        // crop's: a proxy, as the model keeps no herd sizes.
        let mut a = Agents::default();
        let t = |h: f64, d: f64| Traits {
            herding: Traits::from_share(h),
            herd_domestication: Traits::from_share(d),
            ..Traits::FORAGER
        };
        a.push_with(EntityId::new(0, 1), 0, 3, t(0.4, 1.0));
        a.push_with(EntityId::new(0, 2), 0, 1, t(0.0, 0.2));
        let mut out = Agents::default();
        a.merge_small_into(10, &mut out, 0, &mut [0]);
        assert_eq!(out.traits(0), t(0.3, 0.8), "(3 x 0.4 + 1 x 0) / 4, (3 x 1 + 1 x 0.2) / 4");
        let mut reference = a.clone();
        reference.merge_small(10);
        assert_eq!(reference, out);
        let mut sorted = Agents::default();
        sorted.push_with(EntityId::new(0, 2), 0, 1, t(0.0, 0.2));
        sorted.push_with(EntityId::new(0, 1), 0, 3, t(0.4, 1.0));
        sorted.sort_canonical();
        assert_eq!(sorted.traits(0), t(0.4, 1.0), "sorting moves them with their group");
    }

    #[test]
    fn pooled_inequality_does_not_overflow() {
        // Some 4.3 billion people at the largest inequality would overflow a u64 sum.
        let mut pool = Pool::default();
        let t = Traits { inequality: u32::MAX, ..Traits::FORAGER };
        for _ in 0..3 {
            pool.add(2_000_000_000, t);
        }
        assert_eq!(pool.mean().inequality, u32::MAX);
    }

    #[test]
    fn merged_foragers_keep_their_inequality() {
        let mut a = Agents::default();
        let t = Traits { inequality: 200_000_000, ..Traits::FORAGER };
        a.push_with(EntityId::new(0, 1), 0, 3, t);
        a.push_with(EntityId::new(0, 2), 0, 1, t);
        let mut out = Agents::default();
        a.merge_small_into(10, &mut out, 0, &mut [0]);
        assert_eq!(out.traits(0), t, "foragers are not reset to no inequality when they merge");
    }

    #[test]
    fn a_group_founds_its_own_settlement() {
        let mut a = Agents::default();
        a.push(EntityId::new(0, 1), 0, 50);
        assert_eq!(a.settlement, [ALONE]);
    }

    #[test]
    fn a_settlement_pools_its_groups() {
        // Four groups of cell 0: two live together (with one merged away), one alone.
        let t = |e: f64, v: f64| Traits {
            cultivation: Traits::from_share(e),
            inequality: Traits::from_share(v),
            ..Traits::FORAGER
        };
        let mut a = Agents::default();
        a.push_in(EntityId::new(0, 1), 0, 30, t(1.0, 0.4), 7);
        a.push_with(EntityId::new(0, 2), 0, 500, t(0.0, 0.0));
        a.push_in(EntityId::new(0, 3), 0, 10, t(0.6, 0.8), 7);
        a.push_in(EntityId::new(0, 4), 0, 0, t(0.0, 0.0), 7);
        a.push(EntityId::new(0, 5), 0, 20);
        let town = a.settlement_of(0..5, 2);
        assert_eq!(town, a.settlement_of(0..5, 0), "any of its groups names it");
        assert_eq!((town.label, town.first), (7, 0));
        assert_eq!((town.people, town.groups), (40, 2), "groups merged away (no people) are not members");
        assert_eq!(town.cultivation(), Traits::from_share(0.9), "(30 x 1 + 10 x 0.6) / 40");
        assert_eq!(town.inequality(), Traits::from_share(0.5), "(30 x 0.4 + 10 x 0.8) / 40");
        let alone = a.settlement_of(0..5, 1);
        assert_eq!(
            (alone.people, alone.groups, alone.cultivation()),
            (500, 1, 0),
            "not the other lone group's"
        );
        assert!(!a.alone(0..5) && a.alone(1..2));
        let mut seen = vec![];
        a.for_each_settlement(0..5, |s| seen.push((s.first, s.people)));
        assert_eq!(seen, [(0, 40), (1, 500), (4, 20)], "in order of their first group");
    }

    #[test]
    fn a_lone_group_keeps_its_practice_exactly() {
        let mut a = Agents::default();
        let t = Traits { cultivation: 123_456_789, inequality: 987_654_321, ..Traits::FORAGER };
        a.push_with(EntityId::new(0, 1), 0, 77, t);
        let s = a.settlement_of(0..1, 0);
        assert_eq!((s.cultivation(), s.inequality()), (t.cultivation, t.inequality));
    }

    #[test]
    fn merged_small_bands_stay_in_the_absorbers_settlement() {
        let mut a = Agents::default();
        a.push_in(EntityId::new(0, 1), 0, 3, Traits::FORAGER, 9);
        a.push(EntityId::new(0, 2), 0, 1);
        a.push_in(EntityId::new(0, 3), 0, 50, Traits::FORAGER, 9);
        let mut out = Agents::default();
        a.merge_small_into(10, &mut out, 0, &mut [0]);
        assert_eq!(out.settlement, [9, 9]);
        let mut reference = a.clone();
        reference.merge_small(10);
        assert_eq!(reference, out);
    }

    use proptest::prelude::*;

    /// Random agents in `cells` cells with unique ids.
    fn arb_agents(cells: u32) -> impl Strategy<Value = Agents> {
        let row = (0..cells, 0u64..1_000, 0u64..40, 0..=SHARE_ONE, 0..=SHARE_ONE, 0..=u32::MAX, 0u64..4);
        let herds = (0..=SHARE_ONE / 2, 0..=SHARE_ONE, 0..=SHARE_ONE, 0..=SHARE_ONE);
        prop::collection::vec((row, herds), 0..300).prop_map(|rows| {
            let mut a = Agents::default();
            for (i, ((cell, counter, w, e, d, v, s), (h, hd, em, ek))) in rows.into_iter().enumerate() {
                // Unique ids: counter space partitioned by row index.
                let e = e / 2; // effort shares that leave room for herding
                let t = Traits {
                    cultivation: e,
                    domestication: d,
                    inequality: v,
                    herding: h,
                    herd_domestication: hd,
                    wheat_domestication: [em, ek],
                };
                let id = EntityId::new(cell, counter * 1_000 + i as u64);
                // Most groups live alone; the rest in one of a few settlements of their cell.
                a.push_in(id, cell, w, t, s as u32);
            }
            a
        })
    }

    proptest! {
        #[test]
        fn counting_sort_matches_canonical(a in arb_agents(16), base in 0u32..4) {
            // Shift into a partition owning cells base..base+16.
            let mut a = a;
            a.cell.iter_mut().for_each(|c| *c += base);
            let mut expected = a.clone();
            expected.sort_canonical();
            let mut scratch = Agents::default();
            let mut counts = Vec::new();
            a.sort_in_cells(base..base + 16, &mut scratch, &mut counts);
            prop_assert_eq!(a, expected);
        }

        #[test]
        fn fused_merge_matches_reference(a in arb_agents(8), threshold in 0u64..30) {
            let mut expected = a.clone();
            expected.sort_canonical();
            expected.merge_small(threshold);
            let mut expected_pop = vec![0u64; 8];
            for (&c, &w) in expected.cell.iter().zip(&expected.weight) {
                expected_pop[c as usize] += w;
            }

            let mut sorted = a.clone();
            sorted.sort_canonical();
            let mut out = Agents::default();
            let mut pop = vec![0u64; 8];
            sorted.merge_small_into(threshold, &mut out, 0, &mut pop);
            prop_assert_eq!(&out, &expected);
            prop_assert_eq!(pop, expected_pop);
            prop_assert_eq!(out.total_weight(), a.total_weight());
        }
    }

    #[test]
    fn cell_ranges() {
        let mut a = sample();
        a.sort_canonical();
        assert_eq!(a.cell_range(0..1), 0..2);
        assert_eq!(a.cell_range(1..2), 2..5);
    }
}
