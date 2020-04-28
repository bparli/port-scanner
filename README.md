# port-scanner
A simple, yet fast, async port scanner library for Rust.  Built on [async-std](https://github.com/async-rs/async-std)

## Usage
A new Scanner only takes the timeout used for each port.  

To run a port scan against localhost.  This will return a vector of socket addresses that are listening on tcp:

```rust
  use async_std::task;
  use futures::future::join_all;
  use port_scanner::Scanner;
  let ps = Scanner::new(Duration::from_secs(4));

  let ftr = ps.run("127.0.0.1".to_string(), 1, 65535);
  let my_addrs: Vec<SocketAddr> = task::block_on(async { ftr.await });
  println!("{:?}", my_addrs);

```

It's easy to hit the open files limit on your system.  To get around this, limit the scanner to running in batches of ports at a time:

```rust
  let ftr = ps.run_batched("127.0.0.1".to_string(), 1, 65535, 10000);
  let my_addrs: Vec<SocketAddr> = task::block_on(async { ftr.await });
  println!("{:?}", my_addrs);
```

You can also schedule scans against multiple hosts.  This will return a vector of vectors of socket addresses.

```rust
  let my_ftr = ps.run_batched("127.0.0.1".to_string(), 1, 65535, 3000);
  let dev1_ftr = ps.run_batched("192.168.1.172".to_string(), 1, 65535, 3000);
  let dev2_ftr = ps.run_batched("192.168.1.137".to_string(), 1, 65535, 3000);
  let all_ftrs = vec![my_ftr, dev1_ftr, dev2_ftr];
  let results: Vec<Vec<SocketAddr>> = task::block_on(async move { join_all(all_ftrs).await });
  println!("{:?}", results);

```
