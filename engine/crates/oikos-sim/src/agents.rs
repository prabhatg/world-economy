//! Super-individual agents in structure-of-arrays layout.
//!
//! Each agent represents `weight` people (Scheffer et al. 1995). Splitting and merging
//! change resolution while conserving people exactly.

use oikos_core::EntityId;
use std::ops::Range;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Agents {
    pub id: Vec<EntityId>,
    pub cell: Vec<u32>,
    pub weight: Vec<u64>,
}

impl Agents {
    pub fn len(&self) -> usize {
        self.id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.id.is_empty()
    }

    pub fn push(&mut self, id: EntityId, cell: u32, weight: u64) {
        self.id.push(id);
        self.cell.push(cell);
        self.weight.push(weight);
    }

    pub fn append(&mut self, other: &mut Agents) {
        self.id.append(&mut other.id);
        self.cell.append(&mut other.cell);
        self.weight.append(&mut other.weight);
    }

    /// Appends copies of `other`'s agents at indices `span`.
    pub fn extend_from(&mut self, other: &Agents, span: Range<usize>) {
        self.id.extend_from_slice(&other.id[span.clone()]);
        self.cell.extend_from_slice(&other.cell[span.clone()]);
        self.weight.extend_from_slice(&other.weight[span]);
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
    }

    /// Range of agents in `cells` (requires canonical order).
    pub fn cell_range(&self, cells: std::ops::Range<u32>) -> std::ops::Range<usize> {
        let a = self.cell.partition_point(|&c| c < cells.start);
        let b = self.cell.partition_point(|&c| c < cells.end);
        a..b
    }

    /// Removes zero-weight agents and merges every agent below `threshold` in a cell
    /// into the lowest-id small agent of that cell. Requires canonical order and keeps it.
    pub fn merge_small(&mut self, threshold: u64) {
        let mut out = Agents::default();
        let mut i = 0;
        while i < self.len() {
            let cell = self.cell[i];
            let mut absorber: Option<usize> = None;
            while i < self.len() && self.cell[i] == cell {
                let w = self.weight[i];
                if w > 0 && w < threshold {
                    match absorber {
                        Some(a) => out.weight[a] += w,
                        None => {
                            absorber = Some(out.len());
                            out.push(self.id[i], cell, w);
                        }
                    }
                } else if w > 0 {
                    out.push(self.id[i], cell, w);
                }
                i += 1;
            }
        }
        *self = out;
    }

    pub fn clear(&mut self) {
        self.id.clear();
        self.cell.clear();
        self.weight.clear();
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
        // Scatter; afterwards counts[k] is the end of cell k's group.
        for i in 0..n {
            let k = (self.cell[i] - cells.start) as usize;
            let dst = counts[k] as usize;
            counts[k] += 1;
            scratch.id[dst] = self.id[i];
            scratch.cell[dst] = self.cell[i];
            scratch.weight[dst] = self.weight[i];
        }

        let mut start = 0;
        for &end in &counts[..m] {
            let end = end as usize;
            for j in start + 1..end {
                let (id, w) = (scratch.id[j], scratch.weight[j]);
                let mut i = j;
                while i > start && scratch.id[i - 1] > id {
                    scratch.id[i] = scratch.id[i - 1];
                    scratch.weight[i] = scratch.weight[i - 1];
                    i -= 1;
                }
                scratch.id[i] = id;
                scratch.weight[i] = w;
            }
            start = end;
        }
        std::mem::swap(self, scratch);
    }

    /// [`Agents::merge_small`] into `out`, also writing per-cell population to `pop`
    /// (indexed by `cell - cell_base`, zeroed first). Requires canonical order.
    pub fn merge_small_into(&self, threshold: u64, out: &mut Agents, cell_base: u32, pop: &mut [u64]) {
        out.clear();
        pop.iter_mut().for_each(|p| *p = 0);
        let mut i = 0;
        while i < self.len() {
            let cell = self.cell[i];
            let mut absorber: Option<usize> = None;
            let mut total = 0;
            while i < self.len() && self.cell[i] == cell {
                let w = self.weight[i];
                total += w;
                if w > 0 && w < threshold {
                    match absorber {
                        Some(a) => out.weight[a] += w,
                        None => {
                            absorber = Some(out.len());
                            out.push(self.id[i], cell, w);
                        }
                    }
                } else if w > 0 {
                    out.push(self.id[i], cell, w);
                }
                i += 1;
            }
            pop[(cell - cell_base) as usize] = total;
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

    use proptest::prelude::*;

    /// Random agents in `cells` cells with unique ids.
    fn arb_agents(cells: u32) -> impl Strategy<Value = Agents> {
        prop::collection::vec((0..cells, 0u64..1_000, 0u64..40), 0..300).prop_map(|rows| {
            let mut a = Agents::default();
            for (i, (cell, counter, w)) in rows.into_iter().enumerate() {
                // Unique ids: counter space partitioned by row index.
                a.push(EntityId::new(cell, counter * 1_000 + i as u64), cell, w);
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
