use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlamId {
    id: u64,
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

pub struct Match {
    p1: SlamIdQueued,
    p2: SlamIdQueued,
}

impl Match {
    fn new(p1: SlamIdQueued, p2: SlamIdQueued) -> Self {
        Match {
            p1,
            p2,
        }
    }
}

impl std::fmt::Debug for Match {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} vs {}", self.p1.id, self.p2.id)
    }
}

pub struct Slam {
    db: HashMap<SlamId, u64>,
    queue: Vec<SlamIdQueued>,
}

impl Slam {
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

    pub fn poll_queue(&mut self) -> Option<Match> {
        // TODO: better polling, MIP
        if let Some(player) = self.queue.pop() {
            if let Some(matsh) = self.find_best_match(player.into()) {
                let matsh_idx = self.queue.iter().position(|&p| p == matsh).expect("queue should contain SlamIdQueued");
                self.queue.swap_remove(matsh_idx);
                return Some(Match::new(player, matsh));
            }
            self.queue.push(player);
        }
        None
    }

    fn find_best_match(&self, id: SlamId) -> Option<SlamIdQueued> {
        let mut best: Option<SlamIdQueued> = None;
        let mut best_elo_diff = 0u64;
        for player in &self.queue {
            let elo_diff = (*self.db.get(&id).expect("db should contain SlamId") as i64).abs_diff(*self.db.get(&player.into()).expect("db should contain SlamId") as i64);
            if best.is_none() || best_elo_diff > elo_diff {
                best_elo_diff = elo_diff;
                best = Some(*player);
                println!("{}", elo_diff);
            }
        }
        best
    }
}
