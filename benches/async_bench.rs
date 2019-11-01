#![cfg(feature = "kramer-async")]
#![feature(test)]

extern crate async_std;
extern crate test;

use async_std::task;
use kramer::{execute, Arity, Command, Insertion, StringCommand};
use std::env::var;
use test::Bencher;

fn get_redis_url() -> String {
  let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
  let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
  format!("{}:{}", host, port)
}

#[bench]
fn bench_kramer_set_del_async(b: &mut Bencher) {
  b.iter(|| {
    task::block_on(async {
      let key = "kramer_async";
      let mut stream = async_std::net::TcpStream::connect(get_redis_url())
        .await
        .expect("connected");
      let set_cmd = StringCommand::Set(Arity::One((key, "42")), None, Insertion::Always);
      execute(&mut stream, set_cmd).await.expect("written");
      let del_cmd = Command::Del(Arity::One(key));
      execute(&mut stream, del_cmd).await.expect("written");
      Ok::<(), std::io::Error>(())
    })
    .expect("ran async");
  });
}
