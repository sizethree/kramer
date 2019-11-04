#![cfg(feature = "kramer-async")]
#![feature(test)]

extern crate async_std;
extern crate kramer;
extern crate test;

use async_std::task;
use kramer::{execute, Arity, Command, Insertion, Response, ResponseValue, SetCommand, StringCommand};
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
      let del_cmd = Command::Del::<_, &str>(Arity::One(key));
      execute(&mut stream, del_cmd).await.expect("written");
      Ok::<(), std::io::Error>(())
    })
    .expect("ran async");
  });
}

#[bench]
fn bench_kramer_set_operations_async(b: &mut Bencher) {
  b.iter(|| {
    task::block_on(async {
      let (one, two) = ("kramer_async_set_operations_1", "kramer_async_set_operations_2");
      let mut stream = async_std::net::TcpStream::connect(get_redis_url())
        .await
        .expect("connected");

      execute(&mut stream, SetCommand::Add(one, Arity::One("jerry")))
        .await
        .expect("written");

      execute(&mut stream, SetCommand::Add(one, Arity::One("kramer")))
        .await
        .expect("written");

      execute(&mut stream, SetCommand::Add(one, Arity::One("george")))
        .await
        .expect("written");

      execute(&mut stream, SetCommand::Add(two, Arity::One("kramer")))
        .await
        .expect("written");

      let result = execute(&mut stream, SetCommand::Inter::<_, &str>(Arity::Many(vec![one, two])))
        .await
        .expect("written");

      assert_eq!(
        result,
        Response::Array(vec![ResponseValue::String(String::from("kramer"))])
      );

      execute(&mut stream, Command::Del::<_, &str>(Arity::Many(vec![one, two])))
        .await
        .expect("written");
      Ok::<(), std::io::Error>(())
    })
    .expect("ran async");
  });
}
