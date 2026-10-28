//! Mechanism scheduling.
//!
//! Each mechanism declares which state components it reads and writes, how often it
//! runs, and which mechanisms it must run after. [`Schedule::build`] turns that into a
//! deterministic execution order and rejects ambiguity: two mechanisms that touch the
//! same component, where at least one writes it, must be explicitly ordered. Silent
//! order dependence is how simulations become irreproducible.

/// State a mechanism can read or write. Extend at the end; never reorder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
    /// Previous step's population per cell (read-only snapshot during decide).
    Population,
    /// Carrying capacity per cell.
    Capacity,
    /// Agent weights (people per agent).
    AgentWeight,
    /// Agent locations; written by mechanisms that move agents.
    AgentCell,
    /// Creation of new agents (and the per-cell birth counters that name them).
    AgentSpawn,
    /// Forcing-pack fields (read-only inputs: climate, terrain, soils, ...).
    Forcing,
}

/// What a mechanism declares about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechanismSpec {
    pub name: &'static str,
    /// Runs on ticks that are multiples of this period.
    pub period_ticks: u64,
    pub reads: Vec<Component>,
    pub writes: Vec<Component>,
    /// Names of mechanisms that must run before this one.
    pub after: Vec<&'static str>,
}

impl MechanismSpec {
    fn touches(&self, c: Component) -> bool {
        self.reads.contains(&c) || self.writes.contains(&c)
    }
}

/// A validated execution order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    /// Indices into the registered specs, in execution order.
    order: Vec<usize>,
    /// Groups of mechanisms (indices) with no conflicts among them.
    stages: Vec<Vec<usize>>,
    periods: Vec<u64>,
    base_period: u64,
}

impl Schedule {
    pub fn build(specs: &[MechanismSpec]) -> Result<Schedule, String> {
        if specs.is_empty() {
            return Err("schedule needs at least one mechanism".into());
        }
        let index = |name: &str| specs.iter().position(|s| s.name == name);
        for (i, s) in specs.iter().enumerate() {
            if s.period_ticks == 0 {
                return Err(format!("mechanism {}: period_ticks must be > 0", s.name));
            }
            if index(s.name) != Some(i) {
                return Err(format!("duplicate mechanism name {}", s.name));
            }
            for a in &s.after {
                if index(a).is_none() {
                    return Err(format!("mechanism {} runs after unknown mechanism {a}", s.name));
                }
            }
        }

        // Kahn's algorithm; ties broken by registration order for determinism.
        let n = specs.len();
        let preds: Vec<Vec<usize>> =
            specs.iter().map(|s| s.after.iter().map(|a| index(a).unwrap()).collect()).collect();
        let mut placed = vec![false; n];
        let mut order = Vec::with_capacity(n);
        while order.len() < n {
            let next = (0..n).find(|&i| !placed[i] && preds[i].iter().all(|&p| placed[p]));
            match next {
                Some(i) => {
                    placed[i] = true;
                    order.push(i);
                }
                None => {
                    let stuck: Vec<_> = (0..n).filter(|&i| !placed[i]).map(|i| specs[i].name).collect();
                    return Err(format!("dependency cycle among mechanisms {stuck:?}"));
                }
            }
        }

        // Transitive "runs before" relation.
        let mut before = vec![vec![false; n]; n];
        for &i in &order {
            for &p in &preds[i] {
                before[p][i] = true;
                for row in before.iter_mut() {
                    if row[p] {
                        row[i] = true;
                    }
                }
            }
        }
        let ordered = |a: usize, b: usize| before[a][b] || before[b][a];
        let conflict = |a: &MechanismSpec, b: &MechanismSpec| {
            a.writes.iter().copied().find(|&c| b.touches(c)).or(b
                .writes
                .iter()
                .copied()
                .find(|&c| a.touches(c)))
        };
        for a in 0..n {
            for b in a + 1..n {
                if let Some(c) = conflict(&specs[a], &specs[b])
                    && !ordered(a, b)
                {
                    return Err(format!(
                        "mechanisms {} and {} both use {c:?} (at least one writes it) but neither runs \
                         after the other; add an `after` dependency",
                        specs[a].name, specs[b].name
                    ));
                }
            }
        }

        // Stages: each mechanism goes one stage after the latest stage of anything it
        // conflicts with or depends on.
        let mut stage_of = vec![0usize; n];
        for (pos, &i) in order.iter().enumerate() {
            let s = order[..pos]
                .iter()
                .filter(|&&j| before[j][i] || conflict(&specs[i], &specs[j]).is_some())
                .map(|&j| stage_of[j] + 1)
                .max()
                .unwrap_or(0);
            stage_of[i] = s;
        }
        let mut stages: Vec<Vec<usize>> = vec![Vec::new(); stage_of.iter().max().unwrap() + 1];
        for &i in &order {
            stages[stage_of[i]].push(i);
        }

        let periods: Vec<u64> = specs.iter().map(|s| s.period_ticks).collect();
        let base_period = periods.iter().copied().reduce(gcd).unwrap();
        Ok(Schedule { order, stages, periods, base_period })
    }

    /// Registered-spec indices in execution order.
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// Conflict-free groups (future intra-partition parallelism and introspection).
    pub fn stages(&self) -> &[Vec<usize>] {
        &self.stages
    }

    /// The world advances by this many ticks per step: gcd of all periods.
    pub fn base_period(&self) -> u64 {
        self.base_period
    }

    /// Mechanisms (spec indices, execution order) that run at `tick`.
    pub fn due(&self, tick: u64) -> impl Iterator<Item = usize> + '_ {
        self.order.iter().copied().filter(move |&i| tick % self.periods[i] == 0)
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::Component::*;
    use super::*;

    fn spec(
        name: &'static str,
        reads: &[Component],
        writes: &[Component],
        after: &[&'static str],
    ) -> MechanismSpec {
        MechanismSpec {
            name,
            period_ticks: 365,
            reads: reads.to_vec(),
            writes: writes.to_vec(),
            after: after.to_vec(),
        }
    }

    fn names(specs: &[MechanismSpec], idx: impl IntoIterator<Item = usize>) -> Vec<&'static str> {
        idx.into_iter().map(|i| specs[i].name).collect()
    }

    #[test]
    fn orders_by_dependencies_not_registration() {
        let specs = [
            spec("fission", &[Population], &[AgentWeight, AgentSpawn], &["growth"]),
            spec("growth", &[Population, Capacity], &[AgentWeight], &[]),
        ];
        let s = Schedule::build(&specs).unwrap();
        assert_eq!(names(&specs, s.order().iter().copied()), ["growth", "fission"]);
    }

    #[test]
    fn rejects_unordered_write_write() {
        let specs = [spec("a", &[], &[AgentWeight], &[]), spec("b", &[], &[AgentWeight], &[])];
        let e = Schedule::build(&specs).unwrap_err();
        assert!(e.contains("a and b") && e.contains("AgentWeight"), "{e}");
    }

    #[test]
    fn rejects_unordered_read_write() {
        let specs = [spec("reader", &[AgentCell], &[], &[]), spec("mover", &[], &[AgentCell], &[])];
        assert!(Schedule::build(&specs).unwrap_err().contains("AgentCell"));
    }

    #[test]
    fn ordering_is_transitive() {
        let specs = [
            spec("a", &[], &[AgentWeight], &[]),
            spec("b", &[Capacity], &[], &["a"]),
            spec("c", &[AgentWeight], &[], &["b"]),
        ];
        assert!(Schedule::build(&specs).is_ok());
    }

    #[test]
    fn disjoint_mechanisms_share_a_stage() {
        let specs = [
            spec("growth", &[Population], &[AgentWeight], &[]),
            spec("spawner", &[Capacity], &[AgentSpawn], &[]),
            spec("fission", &[], &[AgentWeight], &["growth"]),
        ];
        let s = Schedule::build(&specs).unwrap();
        let stages: Vec<_> = s.stages().iter().map(|g| names(&specs, g.iter().copied())).collect();
        assert_eq!(stages, vec![vec!["growth", "spawner"], vec!["fission"]]);
    }

    #[test]
    fn rejects_cycles_unknown_and_duplicates() {
        let cyc = [spec("a", &[], &[], &["b"]), spec("b", &[], &[], &["a"])];
        assert!(Schedule::build(&cyc).unwrap_err().contains("cycle"));
        let unknown = [spec("a", &[], &[], &["ghost"])];
        assert!(Schedule::build(&unknown).unwrap_err().contains("ghost"));
        let dup = [spec("a", &[], &[], &[]), spec("a", &[], &[], &[])];
        assert!(Schedule::build(&dup).unwrap_err().contains("duplicate"));
        assert!(Schedule::build(&[]).is_err());
        let mut zero = spec("z", &[], &[], &[]);
        zero.period_ticks = 0;
        assert!(Schedule::build(&[zero]).unwrap_err().contains("period"));
    }

    #[test]
    fn multi_rate() {
        let mut yearly = spec("yearly", &[], &[AgentWeight], &[]);
        yearly.period_ticks = 365;
        let mut monthly = spec("monthly", &[Capacity], &[], &[]);
        monthly.period_ticks = 30;
        let specs = [yearly, monthly];
        let s = Schedule::build(&specs).unwrap();
        assert_eq!(s.base_period(), 5);
        assert_eq!(names(&specs, s.due(0)), ["yearly", "monthly"]);
        assert_eq!(names(&specs, s.due(30)), ["monthly"]);
        assert_eq!(names(&specs, s.due(365)), ["yearly"]);
        assert!(s.due(31).next().is_none());
    }
}
