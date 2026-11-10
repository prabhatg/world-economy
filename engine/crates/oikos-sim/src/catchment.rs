//! Catchments (#50): the land people work and the people they live among lie within a
//! distance of where they live, not within a grid cell.
//!
//! Settled farmers worked fields within about an hour's walk of home and foragers ranged
//! within about two (site catchment analysis: 5 and 10 km, Vita-Finzi et al. 1970;
//! Chisholm 1968). A catchment is the block of cells whose land and people are counted
//! together: the cell of a coarser H3 resolution that holds them, at the resolution whose
//! cells come nearest a circle of `catchment_radius_km` in area (each coarser level seven
//! times the area). What mechanisms count per place (people, their practice, what the
//! land feeds, water at hand) is counted per catchment; where people live within one, and
//! the land's detail, stay the grid's. So a grid finer than the catchment adds where
//! villages stand without changing what the parameters mean, and a grid as coarse makes
//! every cell its own catchment, as every cell was before #50.
//!
//! Catchments do not overlap, so each cell's land is counted once: overlapping ones would
//! give the land between two villages to both.
//!
//! A group that leaves its place for new land founds its new home in a neighboring
//! catchment ([`Catchments::away`]), as before #50 in a neighboring cell.
//!
//! Catchments come from the grid's hierarchy and the geometry pack's cell areas, so they
//! are the same on every host; without a geometry pack, every cell is its own.

use crate::environment::AREA;
use crate::inputs::Inputs;
use oikos_grid::Grid;

/// Each cell's catchment and the cells a group leaving it may go to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Catchments {
    /// Per cell, its catchment, numbered in order of first cell (empty when every cell is
    /// its own).
    block: Vec<u32>,
    /// Per catchment, its range in `members`, which holds its cells in order.
    member_offsets: Vec<u32>,
    members: Vec<u32>,
    /// Per cell, its range in `away` holding the cells of [`Catchments::away`].
    away_offsets: Vec<u32>,
    away: Vec<u32>,
}

impl Catchments {
    /// The catchments of `grid`'s cells of area `area_km2(cell)` for a radius of `radius_km`.
    pub fn new(grid: &Grid, area_km2: impl Fn(usize) -> f64, radius_km: f64) -> Catchments {
        let n = grid.len();
        let circle = std::f64::consts::PI * radius_km * radius_km;
        let mean = (0..n).map(&area_km2).sum::<f64>() / n.max(1) as f64;
        if n == 0 || circle <= 0.0 || mean <= 0.0 {
            return Catchments::default();
        }
        // Coarser levels whose cells come nearest the circle's area, by ratio.
        let levels = oikos_math::ln(circle / mean) / oikos_math::ln(7.0) + 0.5;
        let levels = if levels < 1.0 { 0 } else { (levels as u8).min(grid.resolution()) };
        if levels == 0 {
            return Catchments::default();
        }
        let resolution = grid.resolution() - levels;
        let parents: Vec<u64> = (0..n as u32).map(|c| grid.parent(c, resolution)).collect();
        let mut order: Vec<u32> = (0..n as u32).collect();
        order.sort_by_key(|&c| (parents[c as usize], c));
        // Each parent's cells (a run of `order`), numbered in order of their first cell.
        let mut runs: Vec<(u32, usize, usize)> = Vec::new();
        let mut i = 0;
        while i < n {
            let p = parents[order[i] as usize];
            let end = i + order[i..].iter().take_while(|&&c| parents[c as usize] == p).count();
            runs.push((order[i], i, end));
            i = end;
        }
        runs.sort_unstable();
        let mut out = Catchments { block: vec![0; n], ..Catchments::default() };
        for (b, &(_, start, end)) in runs.iter().enumerate() {
            out.member_offsets.push(out.members.len() as u32);
            for &c in &order[start..end] {
                out.block[c as usize] = b as u32;
                out.members.push(c);
            }
        }
        out.member_offsets.push(out.members.len() as u32);
        // The catchments bordering each, in order.
        let bordering: Vec<Vec<u32>> = (0..out.len())
            .map(|b| {
                let mut v: Vec<u32> = out
                    .members(b)
                    .iter()
                    .flat_map(|&m| grid.neighbors(m).iter().map(|&j| out.block[j as usize]))
                    .filter(|&x| x as usize != b)
                    .collect();
                v.sort_unstable();
                v.dedup();
                v
            })
            .collect();
        out.away_offsets.reserve(n + 1);
        // When each cell was last seen: the search from cell `c` marks with `c + 1`.
        let mut seen = vec![0u32; n];
        let (mut frontier, mut ring, mut wanted) = (Vec::new(), Vec::new(), Vec::new());
        for cell in 0..n {
            out.away_offsets.push(out.away.len() as u32);
            let mark = cell as u32 + 1;
            // Each bordering catchment, reached at its cell nearest this one.
            wanted.clear();
            wanted.extend_from_slice(&bordering[out.block[cell] as usize]);
            seen[cell] = mark;
            frontier.clear();
            frontier.push(cell as u32);
            while !wanted.is_empty() {
                ring.clear();
                for &c in &frontier {
                    for &j in grid.neighbors(c) {
                        if seen[j as usize] != mark {
                            seen[j as usize] = mark;
                            ring.push(j);
                        }
                    }
                }
                if ring.is_empty() {
                    break;
                }
                ring.sort_unstable();
                for &j in &ring {
                    if let Ok(i) = wanted.binary_search(&out.block[j as usize]) {
                        wanted.remove(i);
                        out.away.push(j);
                    }
                }
                std::mem::swap(&mut frontier, &mut ring);
            }
        }
        out.away_offsets.push(out.away.len() as u32);
        out
    }

    /// [`Catchments::new`] with the areas of the geometry pack among `inputs`; every cell
    /// its own without one.
    pub fn of_inputs(grid: &Grid, inputs: &Inputs, radius_km: f64) -> Catchments {
        match inputs.field(AREA.0, AREA.1) {
            Ok(area) => Catchments::new(grid, |c| area.get(0, c).unwrap_or(0.0), radius_km),
            Err(_) => Catchments::default(),
        }
    }

    /// Whether every cell is its own catchment.
    pub fn is_trivial(&self) -> bool {
        self.block.is_empty()
    }

    /// The number of catchments (0 when every cell is its own).
    pub fn len(&self) -> usize {
        self.member_offsets.len().saturating_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The catchment of `cell` (not trivial).
    pub fn of(&self, cell: usize) -> usize {
        self.block[cell] as usize
    }

    /// The cells of catchment `block`, in order.
    pub fn members(&self, block: usize) -> &[u32] {
        &self.members[self.member_offsets[block] as usize..self.member_offsets[block + 1] as usize]
    }

    /// The other cells of `cell`'s catchment (none where every cell is its own).
    pub fn mates(&self, cell: usize) -> impl Iterator<Item = usize> + '_ {
        let members = if self.is_trivial() { &[][..] } else { self.members(self.of(cell)) };
        members.iter().map(|&c| c as usize).filter(move |&c| c != cell)
    }

    /// The cells a group leaving `cell` may found its new home in: in each neighboring
    /// catchment, its cell nearest this one (by rings, then in order). Its neighbors, in
    /// order, where every cell is its own catchment.
    pub fn away<'a>(&'a self, grid: &'a Grid, cell: usize) -> &'a [u32] {
        if self.is_trivial() {
            return grid.neighbors(cell as u32);
        }
        &self.away[self.away_offsets[cell] as usize..self.away_offsets[cell + 1] as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> Grid {
        Grid::from_base_cell(22, 3).unwrap()
    }

    fn radius(area: f64) -> f64 {
        (area / std::f64::consts::PI).sqrt()
    }

    #[test]
    fn cells_about_the_circles_size_are_their_own_catchments() {
        let g = grid();
        for (area, circle) in [(4_000.0, 4_000.0), (4_000.0, 10_000.0), (4_000.0, 1_000.0), (100.0, 0.0)] {
            let c = Catchments::new(&g, |_| area, radius(circle));
            assert!(c.is_trivial(), "{area} km2 cells, a {circle} km2 circle");
            assert_eq!(c.mates(5).count(), 0);
            assert_eq!(c.away(&g, 5), g.neighbors(5));
        }
    }

    #[test]
    fn a_catchment_is_the_coarser_cell_nearest_the_circle_in_area() {
        let g = grid();
        // 100 km2 cells: a 700 km2 circle is a resolution-2 cell, 4,900 km2 one of resolution 1.
        for (circle, resolution, blocks) in [(700.0, 2, 49), (500.0, 2, 49), (4_900.0, 1, 7)] {
            let c = Catchments::new(&g, |_| 100.0, radius(circle));
            assert_eq!(c.len(), blocks, "a {circle} km2 circle");
            for cell in 0..g.len() {
                let b = c.of(cell);
                assert!(c.members(b).contains(&(cell as u32)));
                assert!(
                    c.mates(cell)
                        .all(|m| g.parent(m as u32, resolution) == g.parent(cell as u32, resolution))
                );
                assert_eq!(c.mates(cell).count() + 1, c.members(b).len());
            }
            // Numbered in order of their first cell.
            let firsts: Vec<u32> = (0..c.len()).map(|b| c.members(b)[0]).collect();
            assert!(firsts.windows(2).all(|w| w[0] < w[1]));
        }
    }

    #[test]
    fn groups_leave_for_the_nearest_cell_of_each_neighboring_catchment() {
        let g = grid();
        let c = Catchments::new(&g, |_| 100.0, radius(700.0));
        for cell in 0..g.len() {
            let away = c.away(&g, cell);
            let mut blocks: Vec<usize> = away.iter().map(|&j| c.of(j as usize)).collect();
            assert!(!blocks.contains(&c.of(cell)), "never its own catchment");
            blocks.sort_unstable();
            let n = blocks.len();
            blocks.dedup();
            assert_eq!(blocks.len(), n, "one cell in each");
            // Every catchment bordering its own.
            let mut bordering: Vec<usize> = c
                .members(c.of(cell))
                .iter()
                .flat_map(|&m| g.neighbors(m).iter().map(|&j| c.of(j as usize)))
                .filter(|&b| b != c.of(cell))
                .collect();
            bordering.sort_unstable();
            bordering.dedup();
            assert_eq!(blocks, bordering);
        }
        // A cell's own neighbors in another catchment are the nearest of theirs.
        let cell = (0..g.len())
            .find(|&x| g.neighbors(x as u32).iter().any(|&j| c.of(j as usize) != c.of(x)))
            .unwrap();
        for &j in g.neighbors(cell as u32) {
            if c.of(j as usize) != c.of(cell) {
                let there = c.away(&g, cell).iter().find(|&&a| c.of(a as usize) == c.of(j as usize)).unwrap();
                assert!(g.neighbors(cell as u32).contains(there));
            }
        }
    }
}
