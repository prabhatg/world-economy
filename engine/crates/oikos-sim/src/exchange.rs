//! Down-the-line exchange of obsidian (docs/04 §4.2, ADR-0060).
//!
//! Obsidian comes from a few volcanoes and is identifiable by source, so where it ends
//! up traces who passed things to whom. Renfrew, Dixon and Cann (1968) found that its
//! share of Neolithic chipped stone falls off steadily with distance from the source,
//! and Renfrew (1977) showed that down-the-line exchange, in which each group keeps some
//! of what it receives and passes the rest on, produces that fall-off.
//!
//! Each cell holds, for every source group, the availability of its obsidian to the
//! people there (a share: 1 at the source). Every step (a year), people within reach
//! of a source have all they want; in every other occupied cell, availability becomes
//! the mean over the cell and its occupied neighbors in the previous step, less the
//! share `loss` used up or kept along the way. Availability therefore falls off about
//! exponentially along chains of settled cells, and never crosses empty land. Empty
//! cells hold none. It is a tracer: nothing else reads it, so it never changes what
//! people do. The `obsidian` forcing pack says which cells reach which source group.

use crate::agents::{SHARE_ONE, Traits};
use crate::inputs::Inputs;
use oikos_grid::Grid;

/// Kind of the forcing pack whose fields are the obsidian source groups: each field is
/// non-zero in the cells within reach of a source of its group.
pub const OBSIDIAN: &str = "obsidian";

/// Obsidian availability per source group and cell (fixed point, see [`SHARE_ONE`]).
#[derive(Clone, Debug, Default)]
pub struct Obsidian {
    /// Source groups, in the pack's field order.
    pub sources: Vec<String>,
    /// Whether each cell reaches each group's sources, `[source][cell]`. Derived from
    /// the pack, so never checkpointed.
    supplied: Vec<Vec<bool>>,
    /// Availability `[source][cell]`. Checkpointed.
    pub(crate) available: Vec<Vec<u32>>,
    next: Vec<u32>,
}

impl Obsidian {
    /// The source groups of the inputs' obsidian pack, none held yet (no groups without
    /// a pack).
    pub fn new(inputs: &Inputs, cells: usize) -> Obsidian {
        let Ok(pack) = inputs.pack(OBSIDIAN) else {
            return Obsidian::default();
        };
        let mut o = Obsidian::default();
        for f in &pack.header().fields {
            let v = pack.field(&f.name).expect("listed field");
            o.sources.push(f.name.clone());
            o.supplied.push((0..cells).map(|c| v.get(0, c).is_some_and(|x| x > 0.0)).collect());
            o.available.push(vec![0; cells]);
        }
        o
    }

    pub fn active(&self) -> bool {
        !self.sources.is_empty()
    }

    /// One exchange step for every source group, from the cells' populations.
    pub fn exchange(&mut self, grid: &Grid, population: &[u64], loss: f64) {
        for (x, supplied) in self.available.iter_mut().zip(&self.supplied) {
            self.next.resize(x.len(), 0);
            pass_down_the_line(grid, population, supplied, loss, x, &mut self.next);
            std::mem::swap(x, &mut self.next);
        }
    }
}

/// One step of down-the-line exchange of one source group: `new[c]` from `old`
/// (availability, fixed point) and the cells' `population`.
pub fn pass_down_the_line(
    grid: &Grid,
    population: &[u64],
    supplied: &[bool],
    loss: f64,
    old: &[u32],
    new: &mut [u32],
) {
    let keep = 1.0 - loss;
    for c in 0..grid.len() {
        new[c] = if population[c] == 0 {
            0
        } else if supplied[c] {
            SHARE_ONE
        } else {
            let (mut sum, mut k) = (old[c] as u64, 1u64);
            for &n in grid.neighbors(c as u32) {
                if population[n as usize] > 0 {
                    sum += old[n as usize] as u64;
                    k += 1;
                }
            }
            Traits::from_share(keep * (sum as f64 / k as f64) / SHARE_ONE as f64)
        };
    }
}
