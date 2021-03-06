use crate::testing::TestConn;

use std::io::{self, prelude::*, BufRead, BufReader, BufWriter, Lines};
use std::net::{self, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::{fmt, str};

use parking_lot::Mutex;

pub enum ConnError {
    InvalidAddress(net::AddrParseError),
    CannotConnect(io::Error),
}

impl fmt::Display for ConnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConnError::InvalidAddress(e) => write!(f, "invalid address: {}", e),
            ConnError::CannotConnect(e) => write!(f, "cannot connect: {}", e),
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

impl From<TcpConn> for Conn {
    fn from(conn: TcpConn) -> Self {
        Conn::TcpConn(conn)
    }
}

impl From<Arc<TestConn>> for Conn {
    fn from(conn: Arc<TestConn>) -> Self {
        Conn::TestConn(Arc::clone(&conn))
    }
}

pub struct TcpConn {
    reader: Mutex<Lines<BufReader<TcpStream>>>,
    writer: Mutex<BufWriter<TcpStream>>,
}

impl TcpConn {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<Self, ConnError> {
        let conn = TcpStream::connect(&addr).map_err(ConnError::CannotConnect)?;
        debug!("connected");

        let reader = {
            let conn = conn.try_clone().expect("to clone stream");
            Mutex::new(BufReader::new(conn).lines())
        };

        let writer = {
            let conn = conn.try_clone().expect("to clone stream");
            Mutex::new(BufWriter::new(conn))
        };

        Ok(Self { reader, writer })
    }

    pub fn write(&self, data: &str) {
        // XXX: might want to rate limit here
        let mut writer = self.writer.lock();
        for part in split(data) {
            // don't log the password
            if &part[..4] != "PASS" {
                let line = &part[..part.len() - 2];
                trace!("--> {}", &line); // trim the \r\n
            }

            trace!("trying to write to socket");
            if writer.write_all(part.as_bytes()).is_ok() {
                trace!("wrote to socket");
            } else {
                error!("cannot write to socket");
                return;
            }
        }
        writer.flush().expect("to flush");
    }

    pub fn read(&self) -> Option<String> {
        let mut reader = self.reader.lock();
        trace!("trying to read from socket");
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
