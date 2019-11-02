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

#[test]
fn test_srem_single() {
  let key = "test_srem_single";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Rem(key, Arity::One("one"))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_srem_multi() {
  let key = "test_srem_multi";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Rem(key, Arity::Many(vec!["one", "two"]))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_union_single() {
  let key = "test_union_single";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Union(Arity::One(key))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![
      ResponseValue::String(String::from("one")),
      ResponseValue::String(String::from("two")),
    ])
  );
}

#[test]
fn test_union_multi() {
  let (one, two) = ("test_union_multi_1", "test_union_multi_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Union(Arity::Many(vec![one, two]))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(two))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![
      ResponseValue::String(String::from("two")),
      ResponseValue::String(String::from("one")),
    ])
  );
}

#[test]
fn test_scard() {
  let key = "test_scard";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Card(key)).expect("executed");
  execute(&mut con, Command::Del(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_diff_none() {
  let (one, two) = ("test_diff_none_1", "test_diff_none_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("two"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Diff(Arity::Many(vec![one, two]))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(two))).expect("executed");
  assert_eq!(result, Response::Array(vec![]));
}

#[test]
fn test_diff_some() {
  let one = "test_diff_some_1";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Diff(Arity::Many(vec![one]))).expect("executed");
  execute(&mut con, Command::Del(Arity::One(one))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![ResponseValue::String(String::from("one"))])
  );
}
