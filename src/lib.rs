use std::collections::HashMap;

use microlp::{LinearExpr, OptimizationDirection, Problem, ComparisonOp};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlamId {
    pub id: u64,
}

impl SlamId {
    fn new(id: u64) -> Self {
        SlamId {
            id,
        }
    }
}

impl From<SlamIdQueued> for SlamId {
    fn from(value: SlamIdQueued) -> Self {
        SlamId::new(value.id)
    }
}

impl From<&SlamIdQueued> for SlamId {
    fn from(value: &SlamIdQueued) -> Self {
        SlamId::new(value.id)
    }
}


#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlamIdQueued {
    id: u64,
}

impl SlamIdQueued {
    fn new(id: SlamId) -> Self {
        SlamIdQueued {
            id: id.id,
        }
    }
}

pub struct Group<const N: usize> {
    players: [SlamIdQueued; N],
    total_elo: u64,
}

impl<const N: usize> Group<N> {
    pub fn new(players: [SlamIdQueued; N], total_elo: u64) -> Self {
        Group {
            players,
            total_elo,       
        }
    }
}

impl<const N: usize> std::fmt::Debug for Group<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "[")?;
        for (i, player) in self.players.iter().enumerate() {
            write!(f, "{}", player.id)?;
            if i < N - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "] (total_elo: {})", self.total_elo)
    }
}

pub struct Match<const N: usize> {
    g1: Group<N>,
    g2: Group<N>,
}

impl<const N: usize> Match<N> {
    fn new(g1: Group<N>, g2: Group<N>) -> Self {
        Match {
            g1,
            g2,
        }
    }
}

impl<const N: usize> std::fmt::Debug for Match<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?} vs {:?}", self.g1, self.g2)
    }
}

pub struct Slam<const N: usize> {
    pub db: HashMap<SlamId, u64>,
    pub queue: Vec<SlamIdQueued>,
}

impl<const N: usize> Slam<N> {
    pub fn new() -> Self {
        Slam {
            db: HashMap::new(),
            queue: Vec::new(),
        }
    }

    pub fn create_player(&mut self, id: u64) -> SlamId {
        let slam_id = SlamId::new(id);
        self.db.insert(slam_id, 100);
        slam_id
    }

    pub fn insert_player(&mut self, id: u64, elo: u64) -> SlamId {
        let slam_id = SlamId::new(id);
        self.db.insert(slam_id, elo);
        slam_id
    }

    pub fn queue_player(&mut self, id: SlamId) -> SlamIdQueued {
        let queued_id = SlamIdQueued::new(id);
        self.queue.push(queued_id);
        queued_id
    }

    pub fn poll_queue(&mut self) -> Option<Match<N>> {
        let mut problem = Problem::new(OptimizationDirection::Minimize);
        let i = self.queue.len();
        let j = 2usize;

        let xs: Vec<_> = (0..i*j).map(|_| problem.add_binary_var(0.0)).collect();
        let avgs: Vec<_> = (0..j).map(|_| problem.add_var(0.0, (0.0, f64::INFINITY))).collect();
        let z = problem.add_var(1.0, (0.0, f64::INFINITY));

        // sum(j, x(i,j)) <= 1 for all i
        for n in 0..i {
            problem.add_constraint(&[(xs[n], 1.0), (xs[n+i], 1.0)], ComparisonOp::Le, 1.0);
        }
        // sum(i, x(i,j)) = N for all j
        for n in 0..j {
            problem.add_constraint((n*i..i*(n+1)).map(|n2| (xs[n2], 1.0)).collect::<Vec<_>>(), ComparisonOp::Eq, N as f64);
        }
        // avg(j) = sum(i, rating(i)*x(i,j)) / N
        // <=> N*avg(i) - sum(i, rating(i)*x(i,j)) = 0
        for n in 0..j {
            let mut lhs = LinearExpr::empty();
            lhs.add(avgs[n], N as f64);
            for n2 in 0..i {
                lhs.add(xs[n2+i*n], -1.0 * self.db[&self.queue[n2].into()] as f64);
            }
            problem.add_constraint(lhs, ComparisonOp::Eq, 0.0);
        }
        // -z <= avg(1) - avg(0) <= z
        problem.add_constraint(&[(avgs[1], 1.0), (avgs[0], -1.0), (z, 1.0)], ComparisonOp::Ge, 0.0);
        problem.add_constraint(&[(avgs[1], 1.0), (avgs[0], -1.0), (z, -1.0)], ComparisonOp::Le, 0.0);
        // z >= 0
        problem.add_constraint(&[(z, 1.0)], ComparisonOp::Ge, 0.0);
        // avg(j) >= 0 for all j
        for n in 0..j {
            problem.add_constraint(&[(avgs[n], 1.0)], ComparisonOp::Ge, 0.0);
        }

        let solution = problem.solve();
        if let Err(error) = solution {
            match error {
                microlp::Error::Infeasible => (),
                microlp::Error::Unbounded => { 
                    eprintln!("Error while trying to solve the MIP problem (Unbounded)");
                },
                microlp::Error::InternalError(s) => {
                    eprintln!("Error while trying to solve the MIP problem: {}", s);
                }
            };
            return None;
        }
        let solution = solution.unwrap();
        println!("Sol: {}", solution.objective());

        let mut g1_players_idx = Vec::new();
        let mut g2_players_idx = Vec::new();
        let mut all_idx = Vec::new(); // sorted increasing

        for idx in 0..i {
            if solution.var_value_rounded(xs[idx]) == 1.0 {
                g1_players_idx.push(idx);
                all_idx.push(idx);
            } else if solution.var_value_rounded(xs[idx+i]) == 1.0 {
                g2_players_idx.push(idx);
                all_idx.push(idx);
            }
        }

        let g1_players: [SlamIdQueued; N] = g1_players_idx.iter().map(|n| *self.queue.get(*n).expect("player should be in queue")).collect::<Vec<_>>().as_slice().try_into().expect(&format!("{} players should h_ave been found", N));
        let g2_players: [SlamIdQueued; N] = g2_players_idx.iter().map(|n| *self.queue.get(*n).expect("player should be in queue")).collect::<Vec<_>>().as_slice().try_into().expect(&format!("{} players should h_ave been found", N));
        
        // removing reverse order
        for idx in all_idx.iter().rev() {
            self.queue.swap_remove(*idx);
        }

        let g1 = Group::new(g1_players, g1_players.map(|p| *self.db.get(&p.into()).expect("player should be in database")).iter().sum());
        let g2 = Group::new(g1_players, g2_players.map(|p| *self.db.get(&p.into()).expect("player should be in database")).iter().sum());

        Some(Match::new(g1, g2))
    }

    // fn find_best_match(&self, group: Group<1>) -> Option<Group<1>> {
    //     let mut best: Option<Group<1>> = None;
    //     let mut best_elo_diff = 0u64;
    //     for player in &self.queue {
    //         let elo_diff = (*self.db.get(&group.players[0].into()).expect("db should contain SlamId") as i64).abs_diff(*self.db.get(&player.into()).expect("db should contain SlamId") as i64);
    //         if best.is_none() || best_elo_diff > elo_diff {
    //             best_elo_diff = elo_diff;
    //             best = Some(Group { players: [*player] });
    //             println!("{}", elo_diff);
    //         }
    //     }
    //     best
    // }
}
