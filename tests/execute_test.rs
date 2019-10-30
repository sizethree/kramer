extern crate kramer;

use kramer::{send, Arity, Command, Insertion, ListCommand, Response, ResponseValue, Side, StringCommand};
use std::env::var;

fn get_redis_url() -> String {
  let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
  let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
  format!("{}:{}", host, port)
}

#[test]
fn test_send_keys() {
  let url = get_redis_url();
  let result = async_std::task::block_on(send(url.as_str(), Command::Keys("*")));
  assert!(result.is_ok());
}

#[test]
fn test_set_vanilla() {
  let url = get_redis_url();
  let key = "test_set_vanilla";
  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "kramer", None, Insertion::Always)),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("OK")))
  )
}

#[test]
fn test_set_if_not_exists_w_not_exists() {
  let key = "test_set_if_not_exists_w_not_exists";
  let url = get_redis_url();
  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "kramer", None, Insertion::IfNotExists)),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("OK")))
  );
}

#[test]
fn test_set_if_not_exists_w_exists() {
  let key = "test_set_if_not_exists_w_exists";
  let url = get_redis_url();

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "kramer", None, Insertion::Always)),
    )
    .await?;
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "jerry", None, Insertion::IfNotExists)),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Empty));
}

#[test]
fn test_set_if_exists_w_not_exists() {
  let key = "test_set_if_exists_w_not_exists";
  let url = get_redis_url();

  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "kramer", None, Insertion::IfExists)),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Empty));
}

#[test]
fn test_set_if_exists_w_exists() {
  let key = "test_set_if_exists_w_exists";
  let url = get_redis_url();
  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "kramer", None, Insertion::Always)),
    )
    .await?;
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(key, "jerry", None, Insertion::IfExists)),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("OK")))
  );
}

#[test]
fn test_set_with_duration() {
  let (key, url) = ("test_set_duration", get_redis_url());

  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings(StringCommand::Set(
        key,
        "kramer",
        Some(std::time::Duration::new(10, 0)),
        Insertion::Always,
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("OK")))
  )
}

#[test]
fn test_rpush_single() {
  let (key, url) = ("test_rpush_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Right, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_rpush_multiple() {
  let (key, url) = ("test_rpush_many", get_redis_url());

  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Right, Insertion::Always),
        key,
        Arity::Many(vec!["kramer", "jerry"]),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_append() {
  let (key, url) = ("test_append", get_redis_url());

  let result = async_std::task::block_on(async {
    let set_result = send(url.as_str(), Command::Strings(StringCommand::Append(key, "jerry"))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(5)));
}

#[test]
fn test_get() {
  let (key, url) = ("test_get", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), Command::Strings(StringCommand::Append(key, "jerry"))).await?;
    let result = send(url.as_str(), Command::Strings(StringCommand::Get(key))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("jerry")))
  );
}

#[test]
fn test_get_multi_append() {
  let (key, url) = ("test_get_multi_append", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), Command::Strings(StringCommand::Append(key, "jerry"))).await?;
    send(url.as_str(), Command::Strings(StringCommand::Append(key, "kramer"))).await?;
    let result = send(url.as_str(), Command::Strings(StringCommand::Get(key))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("jerrykramer")))
  );
}
