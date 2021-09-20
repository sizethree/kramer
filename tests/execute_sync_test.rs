#![cfg(not(feature = "kramer-async"))]
extern crate kramer;

use kramer::{execute, Arity, AuthCredentials, Command, Insertion, Response, ResponseValue, SetCommand, StringCommand};
use std::env::var;

#[cfg(test)]
fn get_redis_url() -> String {
  let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
  let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
  format!("{}:{}", host, port)
}

// TODO: figure out how to run this in CI; would need to consider how to set password without potentially affecting
// other tests. Might consider a second redis container with auth configured.
#[test]
#[ignore]
fn sync_test_auth_password() {
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let password = var("REDIS_PASSWORD").unwrap_or_default();
  let result = execute(
    &mut con,
    Command::Auth::<String, String>(AuthCredentials::Password(password)),
  )
  .expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::String("OK".into())));
}

#[test]
fn test_strlen_present() {
  let key = "test_strlen_present";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(
    &mut con,
    StringCommand::Set(Arity::One((key, "seinfeld")), None, Insertion::Always),
  )
  .expect("executed");
  let result = execute(&mut con, StringCommand::Len::<_, &str>(key)).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(8)));
}

#[test]
fn test_sadd_single() {
  let key = "test_sadd_single";
  let cmd = SetCommand::Add(key, Arity::One("one"));
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_sadd_multi() {
  let key = "test_sadd_multi";
  let cmd = SetCommand::Add(key, Arity::Many(vec!["one", "two"]));
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_smembers_multi() {
  let key = "test_smembers_multi";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::Many(vec!["one"]))).expect("executed");
  let cmd = SetCommand::Members::<_, &str>(key);
  let result = execute(&mut con, cmd).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
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
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_srem_multi() {
  let key = "test_srem_multi";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Rem(key, Arity::Many(vec!["one", "two"]))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_union_single() {
  let key = "test_union_single";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Union::<_, &str>(Arity::One(key)));
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert!(result.is_ok());
}

#[test]
fn test_union_multi() {
  let (one, two) = ("test_union_multi_1", "test_union_multi_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Union::<_, &str>(Arity::Many(vec![one, two])));
  execute(&mut con, Command::Del::<_, &str>(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(two))).expect("executed");
  assert!(result.is_ok());

  // todo: ordering
  // assert_eq!(
  //   result,
  //   Response::Array(vec![
  //     ResponseValue::String(String::from("two")),
  //     ResponseValue::String(String::from("one")),
  //   ])
  // );
}

#[test]
fn test_scard() {
  let key = "test_scard";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(key, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Card::<_, &str>(key)).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_diff_none() {
  let (one, two) = ("test_diff_none_1", "test_diff_none_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("two"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Diff::<_, &str>(Arity::Many(vec![one, two]))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(two))).expect("executed");
  assert_eq!(result, Response::Array(vec![]));
}

#[test]
fn test_diff_some() {
  let one = "test_diff_some_1";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Diff::<_, &str>(Arity::Many(vec![one]))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(one))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![ResponseValue::String(String::from("one"))])
  );
}

#[test]
fn test_ismember_some() {
  let key = "test_ismember_some";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(key, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::IsMember(key, "one")).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_ismember_none() {
  let key = "test_ismember_none";
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  let result = execute(&mut con, SetCommand::IsMember(key, "one")).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(key))).expect("executed");
  assert_eq!(result, Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_inter_none() {
  let (one, two) = ("test_inter_none_1", "test_inter_none_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("two"))).expect("executed");
  let result = execute(&mut con, SetCommand::Inter::<_, &str>(Arity::Many(vec![one, two]))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(two))).expect("executed");
  assert_eq!(result, Response::Array(vec![]));
}

#[test]
fn test_inter_some() {
  let (one, two) = ("test_inter_some_1", "test_inter_some_2");
  let mut con = std::net::TcpStream::connect(get_redis_url()).expect("connection");
  execute(&mut con, SetCommand::Add(one, Arity::One("one"))).expect("executed");
  execute(&mut con, SetCommand::Add(two, Arity::One("one"))).expect("executed");
  let result = execute(&mut con, SetCommand::Inter::<_, &str>(Arity::Many(vec![one, two]))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(one))).expect("executed");
  execute(&mut con, Command::Del::<_, &str>(Arity::One(two))).expect("executed");
  assert_eq!(
    result,
    Response::Array(vec![ResponseValue::String(String::from("one"))])
  );
}
