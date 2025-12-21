use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use microlp::{LinearExpr, OptimizationDirection, Problem, ComparisonOp};

use serde::Deserialize;

use std::hash::{DefaultHasher, Hash, Hasher};

pub enum Error {
    IoError(std::io::Error),
    SerdeError(serde_json::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::IoError(err) => write!(f, "{:?}", err),
            Error::SerdeError(err) => write!(f, "{:?}", err),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IoError(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::SerdeError(value)
    }
}

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

#[derive(Debug, Deserialize)]
pub struct SlamConfig {
    group_size: u8,
    teams_per_match: u8,
}

impl SlamConfig {
    pub fn new(group_size: u8, teams_per_match: u8) -> Self {
        SlamConfig {
            group_size,
            teams_per_match,
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P)-> Result<Self, Error> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let config: SlamConfig = serde_json::from_reader(reader)?;
        Ok(config)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Group {
    players: Vec<SlamIdQueued>,
    total_elo: u64,
}

impl Group {
    pub fn new(players: Vec<SlamIdQueued>, total_elo: u64) -> Self {
        Group {
            players,
            total_elo,       
        }
    }
}

impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "[")?;
        for (i, player) in self.players.iter().enumerate() {
            write!(f, "{}", player.id)?;
            if i < self.players.len() - 1 {
                write!(f, ", ")?;
            }
        }
        write!(f, "] (total_elo: {})", self.total_elo)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Match {
    g1: Group,
    g2: Group,
}

impl Match {
    fn new(g1: Group, g2: Group) -> Self {
        Match {
            g1,
            g2,
        }
    }

    fn get_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();

        v.extend((self.g1.players.len() as u32).to_be_bytes());
        self.g1.players.iter().map(|id| id.id.to_be_bytes()).for_each(|id_in_bytes| v.extend(id_in_bytes));
        v.extend(self.g1.total_elo.to_be_bytes());

        v.extend((self.g2.players.len() as u32).to_be_bytes());
        self.g2.players.iter().map(|id| id.id.to_be_bytes()).for_each(|id_in_bytes| v.extend(id_in_bytes));
        v.extend(self.g1.total_elo.to_be_bytes());

        v
    }
}

impl std::fmt::Debug for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?} vs {:?}", self.g1, self.g2)
    }
}

pub struct Slam {
    pub db: HashMap<SlamId, u64>,
    pub queue: Vec<SlamIdQueued>,
    ongoing_matches: HashMap<u64, Match>,
    config: SlamConfig,
}

impl Slam {
    pub fn new(config: SlamConfig) -> Self {
        Slam {
            db: HashMap::new(),
            queue: Vec::new(),
            ongoing_matches: HashMap::new(),
            config,
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

    pub fn poll_queue(&mut self) -> Option<Match> {
        let mut problem = Problem::new(OptimizationDirection::Minimize);
        let i = self.queue.len();
        let j = 2usize;
        let N = self.config.group_size;

        let xs: Vec<_> = (0..i*j).map(|_| problem.add_binary_var(0.0)).collect();
        let avgs: Vec<_> = (0..j).map(|_| problem.add_var(0.0, (0.0, f64::INFINITY))).collect();
        let ys: Vec<_> = (0..i*(i-1)*j/2).map(|_| problem.add_binary_var(0.0)).collect();
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
        // map y(i1,i2,j) to x(i1,j)*x(i2,j)
        // y(i1,i2,j) <= x(i1,j)
        // y(i1,i2,j) <= x(i2,j)
        // y(i1,i2,j) >= x(i1,j) + x(i2,j) - 1
        // then
        // y(i1,i2,j)*(rating(i1) - rating(i2)) <= 100 for all i1, i2, j
        let mut acc = 0usize;
        for n in 0..j {
            for n2 in 0..i-1 {
                for n3 in n2+1..i {
                    // ys = [x(0,1,0), ..., x(1,2,0), ..., x(0,0,1), ..., x(i-1,i,j)]
                    //      |________ (i-n2) ________|               |
                    //      |______________ (i*(i-1)/2) _____________|
                    problem.add_constraint(&[(ys[acc], 1.0), (xs[n2+n*i], -1.0)], ComparisonOp::Le, 0.0);
                    problem.add_constraint(&[(ys[acc], 1.0), (xs[n3+n*i], -1.0)], ComparisonOp::Le, 0.0);
                    problem.add_constraint(&[(ys[acc], 1.0), (xs[n2+n*i], -1.0), (xs[n3+n*i], -1.0)], ComparisonOp::Ge, -1.0);
                    let rating_diff = self.db[&self.queue[n2].into()].abs_diff(self.db[&self.queue[n3].into()]) as f64;
                    problem.add_constraint(&[(ys[acc], rating_diff)], ComparisonOp::Le, 200.0);
                    acc += 1;
                }
            }
        }
        // z <= 200 / N (max difference between teams' ELO)
        problem.add_constraint(&[(z, 1.0)], ComparisonOp::Le, 100.0 / N as f64);
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

        let g1_players = g1_players_idx.iter().map(|n| *self.queue.get(*n).expect("player should be in queue")).collect::<Vec<_>>();
        let g2_players = g2_players_idx.iter().map(|n| *self.queue.get(*n).expect("player should be in queue")).collect::<Vec<_>>();
        
        // removing reverse order
        for idx in all_idx.iter().rev() {
            self.queue.swap_remove(*idx);
        }

        let g1_total_elo = g1_players.iter().map(|p| *self.db.get(&p.into()).expect("player should be in database")).sum();
        let g2_total_elo = g2_players.iter().map(|p| *self.db.get(&p.into()).expect("player should be in database")).sum();

        let g1 = Group::new(g1_players, g1_total_elo);
        let g2 = Group::new(g2_players, g2_total_elo);

        Some(Match::new(g1, g2))
    }

    pub fn store_match(&mut self, matsh: Match) {
        self.ongoing_matches.insert(matsh.get_hash(), matsh);
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
