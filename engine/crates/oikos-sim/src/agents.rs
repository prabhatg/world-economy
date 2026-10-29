//! Super-individual agents in structure-of-arrays layout.
//!
//! Each agent represents `weight` people (Scheffer et al. 1995). Splitting and merging
//! change resolution while conserving people exactly. Agents also carry their practice
//! ([`Traits`]): split-off groups inherit it and merged bands take the people-weighted
//! mean, so practice travels with people (demic diffusion).

use oikos_core::EntityId;
use std::ops::Range;

/// Fixed-point one for shares in [0, 1] (parts per billion): exact integers, so merging
/// and checkpoints are the same on every host.
pub const SHARE_ONE: u32 = 1_000_000_000;

/// A band's practice (ADR-0049), as shares of [`SHARE_ONE`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Traits {
    /// Share of subsistence effort spent cultivating.
    pub cultivation: u32,
    /// Share of domestic-type plants in the crop the band sows.
    pub domestication: u32,
}

impl Traits {
    /// Foragers: no cultivation, wild crops.
    pub const FORAGER: Traits = Traits { cultivation: 0, domestication: 0 };

    pub fn share(x: u32) -> f64 {
        x as f64 / SHARE_ONE as f64
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
}

/// People-weighted trait sums of the small bands merging into one absorber. In u64:
/// overflow needs some 18 billion people in small bands of one cell, and u128 is slow
/// on wasm.
#[derive(Default)]
struct Pool {
    weight: u64,
    cultivation: u64,
    domestication: u64,
}

impl Pool {
    fn add(&mut self, w: u64, t: Traits) {
        self.weight += w;
        self.cultivation += w * t.cultivation as u64;
        self.domestication += w * t.domestication as u64;
    }

    /// People-weighted mean traits (rounded down).
    fn mean(&self) -> Traits {
        if self.cultivation == 0 && self.domestication == 0 {
            return Traits::FORAGER;
        }
        let w = self.weight.max(1);
        Traits { cultivation: (self.cultivation / w) as u32, domestication: (self.domestication / w) as u32 }
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

    pub fn push_with(&mut self, id: EntityId, cell: u32, weight: u64, traits: Traits) {
        self.id.push(id);
        self.cell.push(cell);
        self.weight.push(weight);
        self.cultivation.push(traits.cultivation);
        self.domestication.push(traits.domestication);
    }

    pub fn traits(&self, i: usize) -> Traits {
        Traits { cultivation: self.cultivation[i], domestication: self.domestication[i] }
    }

    pub fn set_traits(&mut self, i: usize, t: Traits) {
        self.cultivation[i] = t.cultivation;
        self.domestication[i] = t.domestication;
    }

    pub fn append(&mut self, other: &mut Agents) {
        self.id.append(&mut other.id);
        self.cell.append(&mut other.cell);
        self.weight.append(&mut other.weight);
        self.cultivation.append(&mut other.cultivation);
        self.domestication.append(&mut other.domestication);
    }

    /// Appends copies of `other`'s agents at indices `span`.
    pub fn extend_from(&mut self, other: &Agents, span: Range<usize>) {
        self.id.extend_from_slice(&other.id[span.clone()]);
        self.cell.extend_from_slice(&other.cell[span.clone()]);
        self.weight.extend_from_slice(&other.weight[span.clone()]);
        self.cultivation.extend_from_slice(&other.cultivation[span.clone()]);
        self.domestication.extend_from_slice(&other.domestication[span]);
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
                    out.push_with(self.id[j], cell, w, self.traits(j));
                } else if small.first() == Some(&j) {
                    let people: u64 = small.iter().map(|&k| self.weight[k]).sum();
                    let mean = |x: &[u32]| {
                        let s: u128 = small.iter().map(|&k| self.weight[k] as u128 * x[k] as u128).sum();
                        (s / people as u128) as u32
                    };
                    let t = Traits {
                        cultivation: mean(&self.cultivation),
                        domestication: mean(&self.domestication),
                    };
                    out.push_with(self.id[j], cell, people, t);
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
                let (w, t) = (scratch.weight[j], scratch.traits(j));
                let mut i = j;
                while i > start && scratch.id[i - 1] > id {
                    scratch.id[i] = scratch.id[i - 1];
                    scratch.weight[i] = scratch.weight[i - 1];
                    scratch.cultivation[i] = scratch.cultivation[i - 1];
                    scratch.domestication[i] = scratch.domestication[i - 1];
                    i -= 1;
                }
                scratch.id[i] = id;
                scratch.weight[i] = w;
                scratch.set_traits(i, t);
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
                        out.push_with(self.id[i], cell, 0, Traits::FORAGER);
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
        let t = |e: f64, d: f64| Traits {
            cultivation: Traits::from_share(e),
            domestication: Traits::from_share(d),
        };
        a.push_with(EntityId::new(0, 1), 0, 3, t(1.0, 0.5));
        a.push_with(EntityId::new(0, 2), 0, 1, t(0.0, 0.1));
        a.push_with(EntityId::new(0, 3), 0, 50, t(0.2, 0.2));
        let mut out = Agents::default();
        a.merge_small_into(10, &mut out, 0, &mut [0]);
        assert_eq!(out.len(), 2);
        assert_eq!(out.traits(0), t(0.75, 0.4), "(3 x 1 + 1 x 0) / 4, (3 x 0.5 + 1 x 0.1) / 4");
        assert_eq!(out.traits(1), t(0.2, 0.2), "large bands keep their own");
    }

    use proptest::prelude::*;

    /// Random agents in `cells` cells with unique ids.
    fn arb_agents(cells: u32) -> impl Strategy<Value = Agents> {
        let row = (0..cells, 0u64..1_000, 0u64..40, 0..=SHARE_ONE, 0..=SHARE_ONE);
        prop::collection::vec(row, 0..300).prop_map(|rows| {
            let mut a = Agents::default();
            for (i, (cell, counter, w, e, d)) in rows.into_iter().enumerate() {
                // Unique ids: counter space partitioned by row index.
                let t = Traits { cultivation: e, domestication: d };
                a.push_with(EntityId::new(cell, counter * 1_000 + i as u64), cell, w, t);
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
