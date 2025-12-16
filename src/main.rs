use slam::{Slam};

fn main() {
    println!("Hello, world!");

    let mut slam = Slam::new();

    let p1 = slam.create_player(1624618619);
    let p2 = slam.insert_player(6264146811, 110);
    let p3 = slam.insert_player(3252523521, 150);

    slam.queue_player(p1);
    slam.queue_player(p2);
    slam.queue_player(p3);

    let matchup = slam.poll_queue();
    if matchup.is_some() {
        println!("{:?}", matchup.unwrap());
    } else {
        println!("No matchup!");
    }
}
