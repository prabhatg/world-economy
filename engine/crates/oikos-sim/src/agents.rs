//! Super-individual agents in structure-of-arrays layout.
//!
//! Each agent represents `weight` people (Scheffer et al. 1995). Splitting and merging
//! change resolution while conserving people exactly.

use oikos_core::EntityId;

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
    fn cell_ranges() {
        let mut a = sample();
        a.sort_canonical();
        assert_eq!(a.cell_range(0..1), 0..2);
        assert_eq!(a.cell_range(1..2), 2..5);
    }
}
