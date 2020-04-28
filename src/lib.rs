use async_std::io;
use async_std::net::TcpStream;
use async_std::task;
use futures::stream::{FuturesUnordered, StreamExt};
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

    pub async fn run(&self, host: String, port_start: u32, port_end: u32) -> Vec<SocketAddr> {
        execute(host, port_start, port_end, self.timeout).await
    }

    pub async fn run_batched(
        &self,
        host: String,
        port_start: u32,
        port_end: u32,
        batch: u32,
    ) -> Vec<SocketAddr> {
        let mut begin = port_start;
        let mut end = batch;
        let mut all_addrs = Vec::new();

        while end < port_end {
            let mut batch_addrs = execute(host.clone(), begin, end, self.timeout).await;
            all_addrs.append(&mut batch_addrs);
            begin = end;
            end += batch;
        }
        all_addrs
    }
}

async fn execute(
    host: String,
    port_start: u32,
    port_end: u32,
    timeout: Duration,
) -> Vec<SocketAddr> {
    let mut ftrs = FuturesUnordered::new();
    for port in port_start..port_end {
        ftrs.push(try_connect(host.clone(), port, timeout));
    }

    task::block_on(async {
        let mut open_addrs: Vec<SocketAddr> = Vec::new();
        while let Some(result) = ftrs.next().await {
            match result {
                Ok(addr) => open_addrs.push(addr),
                Err(_) => {}
            }
        }
        open_addrs
    })
}

async fn try_connect(host: String, port: u32, timeout: Duration) -> io::Result<SocketAddr> {
    let addr = host.to_string() + ":" + &port.to_string();
    match addr.parse() {
        Ok(sock_addr) => match connect(sock_addr, timeout).await {
            Ok(stream_result) => {
                match stream_result.shutdown(Shutdown::Both) {
                    _ => {}
                }
                Ok(sock_addr)
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