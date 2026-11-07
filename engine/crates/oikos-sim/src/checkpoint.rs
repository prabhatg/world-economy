//! Canonical binary checkpoints.
//!
//! Layout (all integers little-endian):
//! `magic "OIKOSCK8" | version u32 | region | seed u64 | tick u64 |
//!  params_len u32 | params JSON | cells u32 | forage_capacity u64×cells |
//!  birth_counter u64×cells | soil u32×cells | sources u32 |
//!  (obsidian availability u32×cells)×sources | agents u64 |
//!  (id u64, cell u32, weight u64, cultivation u32, domestication u32,
//!  inequality u32, settlement u32, herding u32, herd_domestication u32,
//!  emmer domestication u32, einkorn domestication u32)×agents`
//!
//! Magic `OIKOSCK7` was the layout before groups sowed emmer and einkorn with their crop
//! (#38); it restores with each as domestic as the crop, which its one crop stood for.
//! Magic `OIKOSCK6` was the layout before groups herded sheep and goats (#37); it restores
//! with no group herding and no domestic herds, which is what it meant. Magic `OIKOSCK5` was the layout before groups lived in settlements of several groups
//! (ADR-0082); it restores with every group living by itself, which is what it meant. Magic `OIKOSCK4` was the layout before agents carried wealth inequality (ADR-0074);
//! it restores with every group at the inequality foragers settle at, from which farmers'
//! moves to their own level within a few decades; nothing else reads it. Magic `OIKOSCK3` was the layout before cells held obsidian (ADR-0060); it restores
//! with none held yet. `OIKOSCK2` was the layout before cells kept soil (ADR-0055); it
//! restores with full soil everywhere, which is what it meant. `OIKOSCK1` was the layout before agents
//! carried traits (ADR-0049). Everything derived (carrying capacity under cultivation,
//! farmland, habitability) is recomputed from the packs, the parameters, the soil and
//! the agents on restore. Parameters missing from a checkpoint were added since it was
//! written; they restore as the model it was written with, by their `Restore` rules
//! (`Params::from_checkpoint`, ADR-0069).
//!
//! The version says how the region is named: 1 = `base_cell u8 | resolution u8`;
//! 2 = `geometry pack id [u8; 32] | packs u32 | pack id [u8; 32]×packs` (all forcing
//! packs, sorted), so a checkpoint only restores with the packs it was made with.
//!
//! The encoding is a pure function of world state, so its BLAKE3 hash is the canonical
//! state hash used to validate results across hosts.

use crate::agents::{Agents, Traits};
use crate::inputs::Inputs;
use crate::params::Params;
use crate::world::{World, WorldConfig};
use oikos_core::{EntityId, Tick};
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"OIKOSCK8";
const MAGIC_BEFORE_WHEATS: &[u8; 8] = b"OIKOSCK7";
const MAGIC_BEFORE_HERDING: &[u8; 8] = b"OIKOSCK6";
const MAGIC_BEFORE_SETTLEMENTS: &[u8; 8] = b"OIKOSCK5";
const MAGIC_BEFORE_INEQUALITY: &[u8; 8] = b"OIKOSCK4";
const MAGIC_BEFORE_OBSIDIAN: &[u8; 8] = b"OIKOSCK3";
const MAGIC_BEFORE_SOIL: &[u8; 8] = b"OIKOSCK2";
const MAGIC_BEFORE_TRAITS: &[u8; 8] = b"OIKOSCK1";
const BASE_CELL: u32 = 1;
const GEOMETRY: u32 = 2;

fn id_bytes(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    assert!(hex.len() == 64, "pack id {hex} is not 64 hex digits");
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("pack id is hex");
    }
    out
}

fn id_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn encode(world: &World) -> Vec<u8> {
    let mut b = Vec::with_capacity(64 + world.capacity.len() * 16 + world.agent_count() * 52);
    b.extend_from_slice(MAGIC);
    match &world.config {
        WorldConfig::BaseCell(r) => {
            b.extend_from_slice(&BASE_CELL.to_le_bytes());
            b.push(r.base_cell);
            b.push(r.resolution);
        }
        WorldConfig::Geometry(r) => {
            b.extend_from_slice(&GEOMETRY.to_le_bytes());
            b.extend_from_slice(&id_bytes(&r.geometry));
            let ids = world.inputs.ids();
            b.extend_from_slice(&(ids.len() as u32).to_le_bytes());
            for id in ids {
                b.extend_from_slice(&id_bytes(id));
            }
        }
    }
    b.extend_from_slice(&world.seed.to_le_bytes());
    b.extend_from_slice(&world.tick.0.to_le_bytes());
    let params = serde_json::to_vec(&world.params).expect("params serialize");
    b.extend_from_slice(&(params.len() as u32).to_le_bytes());
    b.extend_from_slice(&params);
    b.extend_from_slice(&(world.forage_capacity.len() as u32).to_le_bytes());
    for v in &world.forage_capacity {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in &world.birth_counter {
        b.extend_from_slice(&v.to_le_bytes());
    }
    for v in &world.soil {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&(world.obsidian.available.len() as u32).to_le_bytes());
    for source in &world.obsidian.available {
        for v in source {
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b.extend_from_slice(&(world.agent_count() as u64).to_le_bytes());
    for a in world.agent_slices() {
        for i in 0..a.len() {
            b.extend_from_slice(&a.id[i].0.to_le_bytes());
            b.extend_from_slice(&a.cell[i].to_le_bytes());
            b.extend_from_slice(&a.weight[i].to_le_bytes());
            b.extend_from_slice(&a.cultivation[i].to_le_bytes());
            b.extend_from_slice(&a.domestication[i].to_le_bytes());
            b.extend_from_slice(&a.inequality[i].to_le_bytes());
            b.extend_from_slice(&a.settlement[i].to_le_bytes());
            b.extend_from_slice(&a.herding[i].to_le_bytes());
            b.extend_from_slice(&a.herd_domestication[i].to_le_bytes());
            for d in a.wheat_domestication[i] {
                b.extend_from_slice(&d.to_le_bytes());
            }
        }
    }
    b
}

/// Hex BLAKE3 hash of the canonical encoding.
pub fn state_hash(world: &World) -> String {
    blake3::hash(&encode(world)).to_hex().to_string()
}

struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.b.len() < n {
            return Err("checkpoint truncated".into());
        }
        let (a, rest) = self.b.split_at(n);
        self.b = rest;
        Ok(a)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

/// Restores a checkpoint made without forcing packs.
pub fn decode(bytes: &[u8]) -> Result<World, String> {
    decode_with(bytes, Inputs::none())
}

/// Restores a checkpoint; `inputs` must be exactly the forcing packs it records.
pub fn decode_with(bytes: &[u8], inputs: impl Into<Arc<Inputs>>) -> Result<World, String> {
    let inputs = inputs.into();
    let mut r = Reader { b: bytes };
    let kept = match r.take(8)? {
        m if m == MAGIC => (true, true, true, true, true, true),
        m if m == MAGIC_BEFORE_WHEATS => (true, true, true, true, true, false),
        m if m == MAGIC_BEFORE_HERDING => (true, true, true, true, false, false),
        m if m == MAGIC_BEFORE_SETTLEMENTS => (true, true, true, false, false, false),
        m if m == MAGIC_BEFORE_INEQUALITY => (true, true, false, false, false, false),
        m if m == MAGIC_BEFORE_OBSIDIAN => (true, false, false, false, false, false),
        m if m == MAGIC_BEFORE_SOIL => (false, false, false, false, false, false),
        m if m == MAGIC_BEFORE_TRAITS => {
            return Err(
                "checkpoint predates agent traits (format OIKOSCK1); rerun it with this engine".into()
            );
        }
        _ => return Err("not an Oikos checkpoint".into()),
    };
    let (soil_kept, obsidian_kept, inequality_kept, settlement_kept, herding_kept, wheats_kept) = kept;
    let config = match r.u32()? {
        BASE_CELL => WorldConfig::base_cell(r.u8()?, r.u8()?),
        GEOMETRY => {
            let geometry = id_hex(r.take(32)?);
            let n = r.u32()? as usize;
            let want = (0..n).map(|_| r.take(32).map(id_hex)).collect::<Result<Vec<_>, _>>()?;
            if want != inputs.ids() {
                return Err(format!(
                    "checkpoint needs forcing packs [{}], given [{}]",
                    want.join(", "),
                    inputs.ids().join(", ")
                ));
            }
            WorldConfig::geometry(&geometry)
        }
        v => return Err(format!("unsupported checkpoint version {v}")),
    };
    let seed = r.u64()?;
    let tick = Tick(r.u64()?);
    let plen = r.u32()? as usize;
    let params = Params::from_checkpoint(r.take(plen)?)?;
    let grid = Arc::new(config.build_grid(&inputs)?);
    let cells = r.u32()? as usize;
    if cells != grid.len() {
        return Err(format!("checkpoint has {cells} cells, grid has {}", grid.len()));
    }
    let forage_capacity = (0..cells).map(|_| r.u64()).collect::<Result<Vec<_>, _>>()?;
    let birth_counter = (0..cells).map(|_| r.u64()).collect::<Result<Vec<_>, _>>()?;
    let soil = match soil_kept {
        true => (0..cells).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?,
        false => vec![crate::agents::SHARE_ONE; cells],
    };
    if soil.iter().any(|&s| s > crate::agents::SHARE_ONE) {
        return Err("cell soil share above one".into());
    }
    let obsidian = match obsidian_kept {
        true => {
            let sources = r.u32()? as usize;
            let want = inputs.pack(crate::exchange::OBSIDIAN).map_or(0, |p| p.header().fields.len());
            if sources != want {
                return Err(format!(
                    "checkpoint holds obsidian of {sources} source groups, its obsidian pack has {want}"
                ));
            }
            (0..sources)
                .map(|_| (0..cells).map(|_| r.u32()).collect::<Result<Vec<_>, _>>())
                .collect::<Result<Vec<_>, _>>()?
        }
        false => vec![],
    };
    if obsidian.iter().flatten().any(|&x| x > crate::agents::SHARE_ONE) {
        return Err("cell obsidian availability above one".into());
    }
    let n = r.u64()? as usize;
    let foragers = crate::mechanisms::Inequality::foragers(&params);
    let mut agents = Agents::default();
    for _ in 0..n {
        let id = EntityId(r.u64()?);
        let cell = r.u32()?;
        let weight = r.u64()?;
        let (cultivation, domestication) = (r.u32()?, r.u32()?);
        let inequality = if inequality_kept { r.u32()? } else { foragers.inequality };
        let settlement = if settlement_kept { r.u32()? } else { crate::agents::ALONE };
        let (herding, herd_domestication) = if herding_kept { (r.u32()?, r.u32()?) } else { (0, 0) };
        let wheat_domestication = if wheats_kept { [r.u32()?, r.u32()?] } else { [domestication; 2] };
        let traits = Traits {
            cultivation,
            domestication,
            inequality,
            herding,
            herd_domestication,
            wheat_domestication,
        };
        if cell as usize >= cells {
            return Err("agent cell out of range".into());
        }
        let one = crate::agents::SHARE_ONE;
        if [
            traits.cultivation,
            traits.domestication,
            traits.herding,
            traits.herd_domestication,
            traits.wheat_domestication[0],
            traits.wheat_domestication[1],
        ]
        .iter()
        .any(|&x| x > one)
        {
            return Err("agent trait share above one".into());
        }
        if traits.cultivation as u64 + traits.herding as u64 > one as u64 {
            return Err("agent cultivation and herding effort above one".into());
        }
        agents.push_in(id, cell, weight, traits, settlement);
    }
    if !r.b.is_empty() {
        return Err("trailing bytes in checkpoint".into());
    }
    agents.sort_canonical();
    let mut w =
        World::assemble(config, grid, inputs, seed, tick, params, forage_capacity, birth_counter, agents);
    if soil.iter().any(|&s| s < crate::agents::SHARE_ONE) {
        w.restore_soil(soil);
    }
    if obsidian_kept {
        w.restore_obsidian(obsidian);
    }
    Ok(w)
}
