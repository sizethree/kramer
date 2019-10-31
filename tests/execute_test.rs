extern crate kramer;

use kramer::{send, Arity, Command, HashCommand, Insertion, ListCommand, Response, ResponseValue, Side, StringCommand};
use std::env::var;

#[cfg(test)]
fn set_field<S: std::fmt::Display>(key: S, field: S, value: S) -> Command<S> {
  Command::Hashes(HashCommand::Set(key, Arity::One((field, value)), Insertion::Always))
}

#[cfg(test)]
fn get_redis_url() -> String {
  let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
  let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
  format!("{}:{}", host, port)
}

#[test]
fn test_echo() {
  let url = get_redis_url();
  let result = async_std::task::block_on(send(url.as_str(), Command::Echo("hello")));
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String("hello".to_string()))
  );
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
fn test_lpush_single() {
  let (key, url) = ("test_lpush_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_lpush_multi() {
  let (key, url) = ("test_lpush_multi", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::Many(vec!["kramer", "jerry"]),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_lpushx_single_w_no_exists() {
  let (key, url) = ("test_lpushx_single_w_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Left, Insertion::IfExists),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_lpushx_single_w_exists() {
  let (key, url) = ("test_lpushx_single_w_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await?;
    let out = send(
      url.as_str(),
      Command::List(ListCommand::Push(
        (Side::Left, Insertion::IfExists),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
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
fn test_lpop_single() {
  let (key, url) = ("test_lpop_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::List(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry"]),
    ));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::List(ListCommand::Pop(Side::Left, key, None))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("kramer")))
  );
}

#[test]
fn test_rpop_single() {
  let (key, url) = ("test_rpop_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::List(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry"]),
    ));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::List(ListCommand::Pop(Side::Right, key, None))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("jerry")))
  );
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

#[test]
fn test_decr_single() {
  let (key, url) = ("test_decr_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::Strings(StringCommand::Set(key, "3", None, Insertion::Always));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::Strings(StringCommand::Decr(key, 1))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_decrby_single() {
  let (key, url) = ("test_decrby_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::Strings(StringCommand::Set(key, "3", None, Insertion::Always));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::Strings(StringCommand::Decr(key, 2))).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hset_single() {
  let (key, url) = ("test_hset_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hset_multi() {
  let (key, url) = ("test_hset_multi", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes(HashCommand::Set(
      key,
      Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
      Insertion::Always,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_hdel_single() {
  let (key, url) = ("test_hdel_single", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Hashes(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always)),
    )
    .await?;
    let del = Command::Hashes(HashCommand::Del(key, "name", None));
    let result = send(url.as_str(), del).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hdel_multi() {
  let (key, url) = ("test_hdel_multi", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Hashes(HashCommand::Set(
        key,
        Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
        Insertion::Always,
      )),
    )
    .await?;
    let del = Command::Hashes(HashCommand::Del(
      key,
      "name",
      Some(Arity::Many(vec!["name", "friend", "foo"])),
    ));
    let result = send(url.as_str(), del).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_hsetnx_single_w_no_exists() {
  let (key, url) = ("test_hsetnx_single_w_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes(HashCommand::Set(
      key,
      Arity::One(("name", "kramer")),
      Insertion::IfNotExists,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hsetnx_single_w_exists() {
  let (key, url) = ("test_hsetnx_single_w_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let pre_set = Command::Hashes(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always));
    send(url.as_str(), pre_set).await?;
    let do_set = Command::Hashes(HashCommand::Set(
      key,
      Arity::One(("name", "kramer")),
      Insertion::IfNotExists,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_hexists_single() {
  let (key, url) = ("test_hexists_single", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Hashes(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always)),
    )
    .await?;
    let exists = Command::Hashes(HashCommand::Exists(key, "name"));
    let result = send(url.as_str(), exists).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hexists_not_found() {
  let (key, url) = ("test_hexists_not_found", get_redis_url());

  let result = async_std::task::block_on(async {
    let exists = Command::Hashes(HashCommand::Exists(key, "name"));
    let result = send(url.as_str(), exists).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_hgetall_values() {
  let (key, url) = ("test_hgetall_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    send(url.as_str(), set_field(key, "friend", "jerry")).await?;
    let getall = Command::Hashes(HashCommand::Get(key, None));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("name")),
      ResponseValue::String(String::from("kramer")),
      ResponseValue::String(String::from("friend")),
      ResponseValue::String(String::from("jerry")),
    ])
  );
}

#[test]
fn test_hgetall_empty() {
  let (key, url) = ("test_hgetall_empty", get_redis_url());

  let result = async_std::task::block_on(async {
    let getall = Command::Hashes(HashCommand::Get(key, None));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Array(vec![]));
}

#[test]
fn test_hget_values() {
  let (key, url) = ("test_hget_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes(HashCommand::Get(key, Some(Arity::One("name"))));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("kramer"))),
  );
}

#[test]
fn test_hmget_values() {
  let (key, url) = ("test_hmget_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    send(url.as_str(), set_field(key, "friend", "jerry")).await?;
    let getall = Command::Hashes(HashCommand::Get(key, Some(Arity::Many(vec!["name", "friend"]))));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("kramer")),
      ResponseValue::String(String::from("jerry"))
    ]),
  );
}

#[test]
fn test_hlen_values() {
  let (key, url) = ("test_hlen_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes(HashCommand::Len(key));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hlen_no_exists() {
  let (key, url) = ("test_hlen_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let hlen = Command::Hashes(HashCommand::Len(key));
    send(url.as_str(), hlen).await
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_hkeys_values() {
  let (key, url) = ("test_hkeys_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes(HashCommand::Keys(key));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![ResponseValue::String(String::from("name"))])
  );
}

#[test]
fn test_hkeys_no_exists() {
  let (key, url) = ("test_hkeys_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let hlen = Command::Hashes(HashCommand::Keys(key));
    send(url.as_str(), hlen).await
  });

  assert_eq!(result.unwrap(), Response::Array(vec![]));
}
