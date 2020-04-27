use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;
use futures::future::join_all;
use std::io::ErrorKind;
use std::net::{Shutdown, SocketAddr};
use std::time::Duration;

pub struct Scanner {
    timeout: Duration,
}

impl Scanner {
    pub fn new(timeout: Duration) -> Scanner {
        Scanner { timeout: timeout }
    }

    pub async fn run(&self, host: String, port_start: u16, port_end: u16) -> Vec<u16> {
        execute(host, port_start, port_end, self.timeout).await
    }

    pub async fn run_batched(
        &self,
        host: String,
        port_start: u16,
        port_end: u16,
        batch: u16,
    ) -> Vec<u16> {
        let mut begin = port_start;
        let mut end = batch;
        let mut all_ports = Vec::new();

        while end < port_end {
            let mut batch_ports = execute(host.clone(), begin, end, self.timeout).await;
            all_ports.append(&mut batch_ports);
            begin = end;
            end += batch;
        }
        all_ports
    }
}

async fn execute(host: String, port_start: u16, port_end: u16, timeout: Duration) -> Vec<u16> {
    let mut ftrs = Vec::new();
    for port in port_start..port_end {
        ftrs.push(try_connect(host.clone(), port, timeout));
    }

    let mut ports: Vec<_> = join_all(ftrs).await;
    ports.retain(|port| port.is_ok());
    ports
        .into_iter()
        .map(|port| port.unwrap())
        .collect::<Vec<_>>()
}

async fn try_connect(host: String, port: u16, timeout: Duration) -> io::Result<u16> {
    let addr = host.to_string() + ":" + &port.to_string();
    match addr.parse() {
        Ok(sock_addr) => match connect(sock_addr, timeout).await {
            Ok(stream_result) => {
                match stream_result.shutdown(Shutdown::Both) {
                    _ => {}
                }
                Ok(port)
            }
            Err(e) => match e.kind() {
                ErrorKind::Other => {
                    eprintln!("{:?}", e); // in case we get too many open files
                    Err(e)
                }
                _ => Err(io::Error::new(io::ErrorKind::Other, e.to_string())),
            },
        },
        Err(e) => {
            eprintln!("Unable to convert to socket address {:?}", e);
            Err(io::Error::new(io::ErrorKind::Other, e.to_string()))
        }
    }
}

async fn connect(addr: SocketAddr, timeout: Duration) -> io::Result<TcpStream> {
    let stream = io::timeout(timeout, async move { TcpStream::connect(addr).await }).await?;
    Ok(stream)
}
