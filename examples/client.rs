use std::{io::{Error, Read, Write}, net::TcpStream, thread::sleep, time::Duration};

fn main() -> std::io::Result<()> {
    let mut stream = TcpStream::connect("127.0.0.1:8888")?;

    for i in 0..1 {
        stream.write(&[
            0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, i,
            0, 0, 0, 1
        ])?;
        sleep(Duration::from_secs(1));
    }

    let mut buf = [0; 1024];
    loop {
        let n = match stream.read(&mut buf) {
            Ok(0) => return Err(Error::new(std::io::ErrorKind::ConnectionReset, "no more data")),
            Ok(n) => n,
            Err(e) => {
                eprintln!("error while receiving data; err = {:?}", e);
                return Err(e);
            }
        };

        dbg!(buf);
    }
}
