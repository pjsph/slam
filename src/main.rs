use rand::Rng;
use slam::Slam;

fn main() {
    println!("Hello, world!");

    let mut rng = rand::rng();

    let mut slam = Slam::<6>::new();

    for _ in 0..24 {
        let p = slam.insert_player(rng.random(), rng.random_range(100..1000));
        slam.queue_player(p);
    }

    for (k,v) in slam.db.iter() {
        println!("{} => {}", k.id, v);
    }

    for _ in 0..4 {
        let matchup = slam.poll_queue();

        if matchup.is_some() {
            println!("{:?}", matchup.unwrap());
        } else {
            println!("No matchup!");
        }
    }

    // TODO: bind difference between team members' ELO
}
