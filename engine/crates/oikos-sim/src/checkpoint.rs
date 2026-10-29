//! Canonical binary checkpoints.
//!
//! Layout (all integers little-endian):
//! `magic "OIKOSCK2" | version u32 | region | seed u64 | tick u64 |
//!  params_len u32 | params JSON | cells u32 | forage_capacity u64×cells |
//!  birth_counter u64×cells | agents u64 |
//!  (id u64, cell u32, weight u64, cultivation u32, domestication u32)×agents`
//!
//! Magic `OIKOSCK1` was the layout before agents carried traits (ADR-0049). Everything
//! derived (carrying capacity under cultivation, farmland, habitability) is recomputed
//! from the packs, the parameters and the agents on restore.
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

const MAGIC: &[u8; 8] = b"OIKOSCK2";
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
    let mut b = Vec::with_capacity(64 + world.capacity.len() * 16 + world.agent_count() * 28);
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
    b.extend_from_slice(&(world.agent_count() as u64).to_le_bytes());
    for a in world.agent_slices() {
        for i in 0..a.len() {
            b.extend_from_slice(&a.id[i].0.to_le_bytes());
            b.extend_from_slice(&a.cell[i].to_le_bytes());
            b.extend_from_slice(&a.weight[i].to_le_bytes());
            b.extend_from_slice(&a.cultivation[i].to_le_bytes());
            b.extend_from_slice(&a.domestication[i].to_le_bytes());
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
    match r.take(8)? {
        m if m == MAGIC => {}
        m if m == MAGIC_BEFORE_TRAITS => {
            return Err(
                "checkpoint predates agent traits (format OIKOSCK1); rerun it with this engine".into()
            );
        }
        _ => return Err("not an Oikos checkpoint".into()),
    }
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
    let params: Params = serde_json::from_slice(r.take(plen)?).map_err(|e| e.to_string())?;
    let grid = Arc::new(config.build_grid(&inputs)?);
    let cells = r.u32()? as usize;
    if cells != grid.len() {
        return Err(format!("checkpoint has {cells} cells, grid has {}", grid.len()));
    }
    let forage_capacity = (0..cells).map(|_| r.u64()).collect::<Result<Vec<_>, _>>()?;
    let birth_counter = (0..cells).map(|_| r.u64()).collect::<Result<Vec<_>, _>>()?;
    let n = r.u64()? as usize;
    let mut agents = Agents::default();
    for _ in 0..n {
        let id = EntityId(r.u64()?);
        let cell = r.u32()?;
        let weight = r.u64()?;
        let traits = Traits { cultivation: r.u32()?, domestication: r.u32()? };
        if cell as usize >= cells {
            return Err("agent cell out of range".into());
        }
        if traits.cultivation > crate::agents::SHARE_ONE || traits.domestication > crate::agents::SHARE_ONE {
            return Err("agent trait share above one".into());
        }
        agents.push_with(id, cell, weight, traits);
    }
    if !r.b.is_empty() {
        return Err("trailing bytes in checkpoint".into());
    }
    agents.sort_canonical();
    Ok(World::assemble(config, grid, inputs, seed, tick, params, forage_capacity, birth_counter, agents))
}
