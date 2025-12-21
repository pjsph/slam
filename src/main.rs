use slam::{Slam, SlamConfig, Match};

use rand::Rng;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;

const HEADER_SIZE: usize = 4;

trait ServerBoundPacket {
    fn decode(data: &[u8]) -> Self;
}

trait ClientBoundPacket {
    fn encode(&self) -> Vec<u8>;
}

const MATCH_INFO_PACKET_ID: u32 = 0;
#[derive(Debug)]
struct MatchInfoPacket {
    matsh: Match,
}

impl ClientBoundPacket for MatchInfoPacket {
    fn encode(&self) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(MATCH_INFO_PACKET_ID.to_be_bytes());
        v.extend(self.matsh.as_bytes());
        v
    }
}

const MATCH_RESULT_PACKET_ID: u32 = 0;
const MATCH_RESULT_PACKET_SIZE: usize = 8 + 4;
#[derive(Debug)]
struct MatchResultPacket {
    match_id: u64,
    winner: u32,
}

impl ServerBoundPacket for MatchResultPacket {
    fn decode(data: &[u8]) -> Self {
        let match_id = u64::from_be_bytes(data[..8].try_into().unwrap());
        let winner = u32::from_be_bytes(data[8..12].try_into().unwrap());
        MatchResultPacket { match_id, winner }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {
    println!("Hello, world!");

    let config = SlamConfig::load_from_file("config.json").expect("Unable to read config");

    let mut slam = Slam::new(config);

    let mut rng = rand::rng();

    for _ in 0..24 {
        let p = slam.insert_player(rng.random(), rng.random_range(100..1000));
        slam.queue_player(p);
    }

    for (k,v) in slam.db.iter() {
        println!("{} => {}", k.id, v);
    }

    let (tx, rx) = broadcast::channel(16);

    let _polling_task = tokio::task::spawn_blocking(move || {
        loop {
            let matchup = slam.poll_queue();
            
            if let Some(matchup) = matchup {
                tx.send(matchup);
            }
       }
    });

    let listener = TcpListener::bind("127.0.0.1:8888").await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let (mut read, mut write) = socket.into_split();
        // reading task
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let mut bytes_to_read = 0;
            loop {
                let n = match read.read(&mut buf[bytes_to_read..]).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err = {:?}", e);
                        return;
                    },
                };

                println!("received {} bytes", n);

                bytes_to_read += n;
                let mut cursor = 0;

                while bytes_to_read >= HEADER_SIZE {
                    // read packet id
                    let id = u32::from_be_bytes([
                        buf[cursor],
                        buf[cursor+1],
                        buf[cursor+2],
                        buf[cursor+3]
                    ]);
                    let packet_len = get_serverbound_packet_len(id);

                    println!("{} bytes", packet_len);

                    if bytes_to_read < HEADER_SIZE + packet_len {
                        buf.copy_within(cursor..cursor+bytes_to_read, 0);
                        break; // not enough data yet
                    }

                    // handle the message
                    handle_packet(id, &buf[cursor+HEADER_SIZE..cursor+HEADER_SIZE+packet_len]);
                    bytes_to_read -= HEADER_SIZE + packet_len;
                    cursor += HEADER_SIZE + packet_len;
                }
            }
        });
        // writing task
        // for now we broadcast to all peers when a match is found
        let mut rx_cloned = rx.resubscribe();
        tokio::spawn(async move {
            loop {
                let matsh = match rx_cloned.recv().await {
                    Ok(matsh) => matsh,
                    Err(e) => {
                        eprintln!("failed to read match from broadcast; err = {:?}", e);
                        return;
                    }
                };
                let packet = MatchInfoPacket { matsh };
                if let Err(e) = write.write_all(&packet.encode()).await {
                    eprintln!("failed to send match info to client; err = {:?}", e);
                }
            }
        });
    }

    // send match info to client (g1 and g2 players)
    // store hash of Match and Match

    // receive match info from client (Match hash and winner)
    // delete match and update ELOs
}

fn handle_packet(id: u32, data: &[u8]) {
    match id {
        MATCH_RESULT_PACKET_ID => {
            let packet = MatchResultPacket::decode(data);
            dbg!(packet);
            // TODO: store result & update ELOs
        },
        _ => panic!(),
    };
}

fn get_clientbound_packet_len(id: u32) -> usize {
    match id {
        MATCH_INFO_PACKET_ID => todo!(),
        _ => panic!(),
    }
}

fn get_serverbound_packet_len(id: u32) -> usize {
    match id {
        MATCH_RESULT_PACKET_ID => MATCH_RESULT_PACKET_SIZE,
        _ => panic!(),
    }
}
