//! Grid topology on the H3 geodesic hexagon grid.
//!
//! Only integer topology (cell ids, neighbors, hierarchy) is used at simulation time.
//! Geometry (areas, centroids) involves platform trigonometry, so it is computed
//! offline into forcing packs or via `oikos-math`, never from `h3o` inside a run.

use h3o::{BaseCell, CellIndex, Resolution};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug)]
pub enum GridError {
    InvalidBaseCell(u8),
    InvalidResolution(u8),
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::InvalidBaseCell(b) => write!(f, "invalid H3 base cell {b}"),
            GridError::InvalidResolution(r) => write!(f, "invalid H3 resolution {r}"),
        }
    }
}

impl std::error::Error for GridError {}

/// A region of H3 cells at one resolution with neighbor lists restricted to the region.
///
/// Cells are stored in ascending H3 index order, which follows the H3 hierarchy and
/// keeps nearby cells close together; local indices (`u32`) are positions in that order.
pub struct Grid {
    resolution: Resolution,
    cells: Vec<CellIndex>,
    neighbor_offsets: Vec<u32>,
    neighbor_targets: Vec<u32>,
}

impl Grid {
    /// All descendants at `resolution` of one of the 122 H3 base cells.
    pub fn from_base_cell(base_cell: u8, resolution: u8) -> Result<Grid, GridError> {
        let base = BaseCell::try_from(base_cell).map_err(|_| GridError::InvalidBaseCell(base_cell))?;
        let res = Resolution::try_from(resolution).map_err(|_| GridError::InvalidResolution(resolution))?;
        let root = CellIndex::base_cells()
            .find(|c| c.base_cell() == base)
            .ok_or(GridError::InvalidBaseCell(base_cell))?;
        Ok(Grid::from_cells(res, root.children(res).collect()))
    }

    /// Builds a grid from arbitrary cells (all at `resolution`).
    pub fn from_cells(resolution: Resolution, mut cells: Vec<CellIndex>) -> Grid {
        cells.sort_unstable_by_key(|c| u64::from(*c));
        cells.dedup();
        let index: HashMap<CellIndex, u32> = cells.iter().enumerate().map(|(i, c)| (*c, i as u32)).collect();

        let mut neighbor_offsets = Vec::with_capacity(cells.len() + 1);
        let mut neighbor_targets = Vec::with_capacity(cells.len() * 6);
        neighbor_offsets.push(0);
        for cell in &cells {
            let mut ns: Vec<u32> = cell
                .grid_disk::<Vec<_>>(1)
                .into_iter()
                .filter(|n| n != cell)
                .filter_map(|n| index.get(&n).copied())
                .collect();
            ns.sort_unstable();
            neighbor_targets.extend(ns);
            neighbor_offsets.push(neighbor_targets.len() as u32);
        }
        Grid { resolution, cells, neighbor_offsets, neighbor_targets }
    }

    pub fn resolution(&self) -> u8 {
        u8::from(self.resolution)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn h3(&self, cell: u32) -> u64 {
        u64::from(self.cells[cell as usize])
    }

    /// Neighbors of `cell` inside the region, in ascending local-index order.
    pub fn neighbors(&self, cell: u32) -> &[u32] {
        let a = self.neighbor_offsets[cell as usize] as usize;
        let b = self.neighbor_offsets[cell as usize + 1] as usize;
        &self.neighbor_targets[a..b]
    }

    /// Splits cells into `parts` contiguous ranges of near-equal size.
    pub fn partition(&self, parts: usize) -> Vec<Range<u32>> {
        partition_ranges(self.len() as u32, parts)
    }
}

/// `n` items into `parts` contiguous ranges whose sizes differ by at most one.
pub fn partition_ranges(n: u32, parts: usize) -> Vec<Range<u32>> {
    assert!(parts > 0);
    let parts = parts as u32;
    (0..parts).map(|p| (n * p / parts)..(n * (p + 1) / parts)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn southwest_asia_region() {
        // Base cell 22 covers the Fertile Crescent (36°N, 40°E).
        let g = Grid::from_base_cell(22, 4).unwrap();
        assert_eq!(g.len(), 2401);
        for c in 0..g.len() as u32 {
            let ns = g.neighbors(c);
            assert!(ns.len() <= 6 && !ns.is_empty());
            for &n in ns {
                assert!(g.neighbors(n).contains(&c), "neighbor relation must be symmetric");
            }
        }
        // Interior cells have six neighbors; only the region boundary has fewer.
        let interior = (0..g.len() as u32).filter(|&c| g.neighbors(c).len() == 6).count();
        assert!(interior > g.len() * 9 / 10, "interior = {interior}");
    }

    #[test]
    fn partitions_cover() {
        let r = partition_ranges(10, 3);
        assert_eq!(r, vec![0..3, 3..6, 6..10]);
    }
}
