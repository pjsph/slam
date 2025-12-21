use rand::Rng;
use slam::{Slam, SlamConfig};

fn main() {
    println!("Hello, world!");

    let mut rng = rand::rng();

    let config = SlamConfig::load_from_file("config.json").expect("Unable to read config");

    let mut slam = Slam::new(config);

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
            println!("{:?}", matchup.as_ref().unwrap());
        } else {
            println!("No matchup!");
        }
    }

    // send match info to client (g1 and g2 players)
    // store hash of Match and Match

    // receive match info from client (Match hash and winner)
    // delete match and update ELOs
}
