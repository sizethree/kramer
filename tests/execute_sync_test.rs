#![cfg(not(feature = "kramer-async"))]
extern crate kramer;

use kramer::{execute, Arity, Command, Response, ResponseValue, SetCommand};
use std::env::var;

#[cfg(test)]
fn get_redis_url() -> String {
  let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
  let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
  format!("{}:{}", host, port)
}

#[test]
fn test_sadd_single() {
  let key = "test_sadd_single";
  let cmd = SetCommand::Add(key, Arity::One("one"));
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_sadd_multi() {
  let key = "test_sadd_multi";
  let cmd = SetCommand::Add(key, Arity::Many(vec!["one", "two"]));
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_smembers_multi() {
  let key = "test_smembers_multi";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::Many(vec!["one"]))).expect("executed");
  let cmd = SetCommand::Members(key);
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![ResponseValue::String(String::from("one")),])
  );
}
