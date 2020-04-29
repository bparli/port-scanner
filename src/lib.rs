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
        let mut end = begin + batch;
        let mut all_addrs = Vec::new();

        while end <= port_end {
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

#[cfg(test)]
mod tests {
    use async_std::io;
    use async_std::net::TcpListener;
    use async_std::task;
    use futures::future::join_all;
    use std::net::SocketAddr;
    use std::time::Duration;

    #[test]
    fn test_run() -> io::Result<()> {
        task::block_on(async {
            let sock_addr1 = "127.0.0.1:8080".parse().unwrap();
            let sock_addr2 = "127.0.0.1:8081".parse().unwrap();
            let listener1 = TcpListener::bind(sock_addr1).await?;
            let listener2 = TcpListener::bind(sock_addr2).await?;
            task::spawn(async move { listener1.accept().await });
            task::spawn(async move { listener2.accept().await });

            use super::Scanner;
            let ps = Scanner::new(Duration::from_secs(4));

            let ftr = ps.run("127.0.0.1".to_string(), 8000, 8100);
            let my_addrs: Vec<SocketAddr> = task::block_on(async { ftr.await });
            assert_eq!(my_addrs, vec![sock_addr1, sock_addr2]);
            Ok(())
        })
    }

    #[test]
    fn test_batched() -> io::Result<()> {
        task::block_on(async {
            let sock_addr1 = "127.0.0.1:9080".parse().unwrap();
            let sock_addr2 = "127.0.0.1:9081".parse().unwrap();
            let listener1 = TcpListener::bind(sock_addr1).await?;
            let listener2 = TcpListener::bind(sock_addr2).await?;
            task::spawn(async move { listener1.accept().await });
            task::spawn(async move { listener2.accept().await });

            use super::Scanner;
            let ps = Scanner::new(Duration::from_secs(4));

            let ftr = ps.run_batched("127.0.0.1".to_string(), 9000, 9100, 25);
            let my_addrs: Vec<SocketAddr> = task::block_on(async { ftr.await });
            assert_eq!(my_addrs, vec![sock_addr1, sock_addr2]);
            Ok(())
        })
    }

    #[test]
    fn test_multiple_futures() -> io::Result<()> {
        task::block_on(async {
            let sock_addr1 = "127.0.0.1:5000".parse().unwrap();
            let sock_addr2 = "127.0.0.1:5050".parse().unwrap();
            let sock_addr3 = "127.0.0.1:5051".parse().unwrap();
            let sock_addr4 = "127.0.0.1:5052".parse().unwrap();
            let listener1 = TcpListener::bind(sock_addr1).await?;
            let listener2 = TcpListener::bind(sock_addr2).await?;
            let listener3 = TcpListener::bind(sock_addr3).await?;
            let listener4 = TcpListener::bind(sock_addr4).await?;
            task::spawn(async move { listener1.accept().await });
            task::spawn(async move { listener2.accept().await });
            task::spawn(async move { listener3.accept().await });
            task::spawn(async move { listener4.accept().await });

            use super::Scanner;
            let ps = Scanner::new(Duration::from_secs(4));

            let ftr1 = ps.run("127.0.0.1".to_string(), 5000, 5025);
            let ftr2 = ps.run("127.0.0.1".to_string(), 5025, 5100);
            let all_ftrs = vec![ftr1, ftr2];
            let my_addrs: Vec<Vec<SocketAddr>> = task::block_on(async { join_all(all_ftrs).await });
            assert_eq!(
                my_addrs,
                vec![vec![sock_addr1], vec![sock_addr2, sock_addr3, sock_addr4]]
            );
            Ok(())
        })
    }
}
