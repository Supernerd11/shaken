use testing::TestConn;

use std::io::{self, prelude::*, BufRead, BufReader, BufWriter, Lines};
use std::net::{self, TcpStream, ToSocketAddrs};
use std::sync::{Arc, RwLock};
use std::{fmt, str};

pub enum ConnError {
    InvalidAddress(net::AddrParseError),
    CannotConnect(io::Error),
}

impl fmt::Display for ConnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnError::InvalidAddress(e) => writeln!(f, "invalid address: {}", e),
            ConnError::CannotConnect(e) => writeln!(f, "cannot connect: {}", e),
        }
    }
}

pub enum Conn {
    TcpConn(TcpConn),
    TestConn(Arc<TestConn>),
}

impl Conn {
    pub fn read(&self) -> Option<String> {
        match *self {
            Conn::TcpConn(ref conn) => conn.read(),
            Conn::TestConn(ref conn) => conn.read(),
        }
    }

    pub fn write(&self, data: &str) {
        match *self {
            Conn::TcpConn(ref conn) => conn.write(data),
            Conn::TestConn(ref conn) => conn.write(data),
        }
    }
}

pub struct TcpConn {
    reader: RwLock<Lines<BufReader<TcpStream>>>,
    writer: RwLock<BufWriter<TcpStream>>,
}

impl TcpConn {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, ConnError> {
        let conn = TcpStream::connect(&addr).map_err(ConnError::CannotConnect)?;
        debug!("connected");

        let reader = {
            let conn = conn.try_clone().expect("to clone stream");
            RwLock::new(BufReader::new(conn).lines())
        };

        let writer = {
            let conn = conn.try_clone().expect("to clone stream");
            RwLock::new(BufWriter::new(conn))
        };

        Ok(Self { reader, writer })
    }

    pub fn run(&self, process: fn(String)) {
        trace!("starting run loop");
        while let Some(line) = self.read() {
            trace!("<-- {}", line);
            process(line)
        }
        trace!("end of run loop");
    }

    pub fn write(&self, data: &str) {
        // XXX: might want to rate limit here
        let mut writer = self.writer.write().unwrap();
        for part in split(data) {
            // don't log the password
            if &part[..4] != "PASS" {
                let line = &part[..part.len() - 2];
                trace!("--> {}", &line); // trim the \r\n
            }

            let _ = writer.write_all(part.as_bytes());
        }
        let _ = writer.flush();
    }

    pub fn read(&self) -> Option<String> {
        let mut reader = self.reader.write().unwrap();
        if let Some(Ok(line)) = reader.next() {
            trace!("<-- {}", &line);
            Some(line)
        } else {
            warn!("couldn't read line");
            None
        }
    }
}

fn split<S: AsRef<str>>(raw: S) -> Vec<String> {
    let raw = raw.as_ref();

    if raw.len() > 510 - 2 && raw.contains(':') {
        let split = raw.splitn(2, ':').map(|s| s.trim()).collect::<Vec<_>>();
        let (head, tail) = (split[0], split[1]);
        let mut vec = vec![];
        for part in tail
            .as_bytes()
            .chunks(510 - head.len() - 2)
            .map(str::from_utf8)
        {
            match part {
                Ok(part) => vec.push(format!("{} :{}\r\n", head, part)),
                Err(err) => {
                    warn!("dropping a slice: {}", err);
                    continue;
                }
            }
        }
        vec
    } else {
        vec![format!("{}\r\n", raw)]
    }
}
