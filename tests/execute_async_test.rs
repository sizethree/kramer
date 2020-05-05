#![cfg(feature = "kramer-async")]

extern crate kramer;

use async_std::prelude::*;

use kramer::{
  execute, read, send, Arity, Command, HashCommand, Insertion, ListCommand, Response, ResponseValue, Side,
  StringCommand,
};
use std::env::var;

#[cfg(test)]
fn set_field<S: std::fmt::Display>(key: S, field: S, value: S) -> Command<S, S> {
  Command::Hashes::<_, _>(HashCommand::Set(key, Arity::One((field, value)), Insertion::Always))
}

#[cfg(test)]
fn arity_single_pair<S: std::fmt::Display>(key: S, value: S) -> Arity<(S, S)> {
  Arity::One((key, value))
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
  let result = async_std::task::block_on(send(url.as_str(), Command::Echo::<_, &str>("hello")));
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String("hello".to_string()))
  );
}

#[test]
fn test_execute() {
  let url = get_redis_url();
  let result = async_std::task::block_on(async {
    let mut stream = async_std::net::TcpStream::connect(url).await?;
    execute(&mut stream, Command::Echo::<_, &str>("hello")).await
  });
  assert_eq!(result.is_ok(), true);
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("hello")))
  );
}
#[test]
fn test_execute_arc() {
  let url = get_redis_url();
  let stream = async_std::task::block_on(async_std::net::TcpStream::connect(url)).unwrap();
  let arc = async_std::sync::Arc::new(async_std::sync::Mutex::new(stream));

  let result = async_std::task::block_on(async {
    let (a, b) = (arc.clone(), arc.clone());

    let one = async_std::task::spawn(async move {
      let mut conn = a.lock().await;
      execute(&mut (*conn), Command::Echo::<_, &str>("hello")).await
    });

    let two = async_std::task::spawn(async move {
      let mut conn = b.lock().await;
      execute(&mut (*conn), Command::Echo::<_, &str>("world")).await
    });

    (one.await.unwrap(), two.await.unwrap())
  });

  assert_eq!(
    result,
    (
      Response::Item(ResponseValue::String(String::from("hello"))),
      Response::Item(ResponseValue::String(String::from("world")))
    )
  );
}

#[test]
fn test_execute_nested() {
  let result = async_std::task::block_on(async {
    async_std::task::spawn(async {
      let url = get_redis_url();
      send(url.as_str(), Command::Echo::<_, &str>("hello")).await
    })
    .await
  });
  assert_eq!(result.is_ok(), true);
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("hello")))
  );
}

#[test]
fn test_send_keys() {
  let url = get_redis_url();
  let result = async_std::task::block_on(send(url.as_str(), Command::Keys::<_, &str>("*")));
  assert!(result.is_ok());
}

#[test]
fn test_set_vanilla() {
  let url = get_redis_url();
  let key = "test_set_vanilla";
  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Set(Arity::One((key, "kramer")), None, Insertion::Always)),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Strings::<_, &str>(StringCommand::Set(
        Arity::One((key, "kramer")),
        None,
        Insertion::IfNotExists,
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Strings::<_, &str>(StringCommand::Set(Arity::One((key, "kramer")), None, Insertion::Always)),
    )
    .await?;
    let set_result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Set(
        arity_single_pair(key, "jerry"),
        None,
        Insertion::IfNotExists,
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Strings::<_, &str>(StringCommand::Set(
        arity_single_pair(key, "kramer"),
        None,
        Insertion::IfExists,
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Strings::<_, &str>(StringCommand::Set(
        arity_single_pair(key, "kramer"),
        None,
        Insertion::Always,
      )),
    )
    .await?;
    let set_result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Set(
        arity_single_pair(key, "jerry"),
        None,
        Insertion::IfExists,
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Strings::<_, &str>(StringCommand::Set(
        arity_single_pair(key, "kramer"),
        Some(std::time::Duration::new(10, 0)),
        Insertion::Always,
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    set_result
  });
  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("OK")))
  )
}

#[test]
fn test_blpush_single() {
  let (key, url) = ("test_lpush_single", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await?;
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Pop(Side::Left, key, Some((None, 0)))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from(key)),
      ResponseValue::String(String::from("kramer"))
    ])
  );
}

#[test]
fn test_blpush_blocking() {
  let (key, url) = ("test_lpush_blocking", get_redis_url());

  let handle = async_std::task::spawn(async {
    let cmd = Command::List::<_, &str>(ListCommand::Pop(Side::Left, "test_lpush_blocking", Some((None, 0))));
    let url = get_redis_url();
    let dest = url.as_str();
    let mut con = async_std::net::TcpStream::connect(dest).await.expect("foo");
    let f = format!("{}", cmd);
    con.write_all(f.as_bytes()).await.expect("wrote command");
    read(con).await.expect("read response from redis")
  });

  async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await
    .expect("pushed");
  });

  let result = async_std::task::block_on(handle);

  assert_eq!(
    result,
    Response::Array(vec![
      ResponseValue::String(String::from(key)),
      ResponseValue::String(String::from("kramer"))
    ])
  );
}

#[test]
fn test_lpush_single() {
  let (key, url) = ("test_lpush_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_llen_single() {
  let (key, url) = ("test_llen_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Left, Insertion::Always),
      key,
      Arity::One("kramer"),
    ));
    send(url.as_str(), ins).await?;
    let result = send(url.as_str(), Command::List::<_, &str>(ListCommand::Len(key))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_lpush_multi() {
  let (key, url) = ("test_lpush_multi", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::Many(vec!["kramer", "jerry"]),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::IfExists),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await?;
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Left, Insertion::IfExists),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Right, Insertion::Always),
        key,
        Arity::One("kramer"),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_lpop_single() {
  let (key, url) = ("test_lpop_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::List::<_, &str>(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry"]),
    ));
    send(url.as_str(), push).await?;
    let result = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Pop(Side::Left, key, None)),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let push = Command::List::<_, &str>(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry"]),
    ));
    send(url.as_str(), push).await?;
    let result = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Pop(Side::Right, key, None)),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::List::<_, &str>(ListCommand::Push(
        (Side::Right, Insertion::Always),
        key,
        Arity::Many(vec!["kramer", "jerry"]),
      )),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_append() {
  let (key, url) = ("test_append", get_redis_url());

  let result = async_std::task::block_on(async {
    let set_result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(key, "jerry")),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    set_result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(5)));
}

#[test]
fn test_get() {
  let (key, url) = ("test_get", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(key, "jerry")),
    )
    .await?;
    let result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Get(Arity::One(key))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(key, "jerry")),
    )
    .await?;
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(key, "kramer")),
    )
    .await?;
    let result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Get(Arity::One(key))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("jerrykramer")))
  );
}

#[test]
fn test_multi_get() {
  let (one, two, url) = ("test_multi_get_1", "test_multi_get_2", get_redis_url());

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(one, "jerry")),
    )
    .await?;
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Append(two, "kramer")),
    )
    .await?;
    let result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Get(Arity::Many(vec![one, two]))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(one))).await?;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(two))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("jerry")),
      ResponseValue::String(String::from("kramer")),
    ]),
  );
}

#[test]
fn test_decr_single() {
  let (key, url) = ("test_decr_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::Strings::<_, &str>(StringCommand::Set(arity_single_pair(key, "3"), None, Insertion::Always));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::Strings::<_, &str>(StringCommand::Decr(key, 1))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_decrby_single() {
  let (key, url) = ("test_decrby_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let push = Command::Strings::<_, &str>(StringCommand::Set(arity_single_pair(key, "3"), None, Insertion::Always));
    send(url.as_str(), push).await?;
    let result = send(url.as_str(), Command::Strings::<_, &str>(StringCommand::Decr(key, 2))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hset_single() {
  let (key, url) = ("test_hset_single", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes::<_, &str>(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hset_multi() {
  let (key, url) = ("test_hset_multi", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes::<_, &str>(HashCommand::Set(
      key,
      Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
      Insertion::Always,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Hashes::<_, &str>(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always)),
    )
    .await?;
    let del = Command::Hashes::<_, &str>(HashCommand::Del(key, Arity::One("name")));
    let result = send(url.as_str(), del).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Hashes::<_, &str>(HashCommand::Set(
        key,
        Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
        Insertion::Always,
      )),
    )
    .await?;
    let del = Command::Hashes::<_, &str>(HashCommand::Del(key, Arity::Many(vec!["name", "friend", "foo"])));
    let result = send(url.as_str(), del).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(2)));
}

#[test]
fn test_hsetnx_single_w_no_exists() {
  let (key, url) = ("test_hsetnx_single_w_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Hashes::<_, &str>(HashCommand::Set(
      key,
      Arity::One(("name", "kramer")),
      Insertion::IfNotExists,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hsetnx_single_w_exists() {
  let (key, url) = ("test_hsetnx_single_w_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let pre_set = Command::Hashes::<_, &str>(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always));
    send(url.as_str(), pre_set).await?;
    let do_set = Command::Hashes::<_, &str>(HashCommand::Set(
      key,
      Arity::One(("name", "kramer")),
      Insertion::IfNotExists,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
      Command::Hashes::<_, &str>(HashCommand::Set(key, Arity::One(("name", "kramer")), Insertion::Always)),
    )
    .await?;
    let exists = Command::Hashes::<_, &str>(HashCommand::Exists(key, "name"));
    let result = send(url.as_str(), exists).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hexists_not_found() {
  let (key, url) = ("test_hexists_not_found", get_redis_url());

  let result = async_std::task::block_on(async {
    let exists = Command::Hashes::<_, &str>(HashCommand::Exists(key, "name"));
    let result = send(url.as_str(), exists).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let getall = Command::Hashes::<_, &str>(HashCommand::Get(key, None));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let getall = Command::Hashes::<_, &str>(HashCommand::Get(key, None));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Array(vec![]));
}

#[test]
fn test_hget_values() {
  let (key, url) = ("test_hget_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes::<_, &str>(HashCommand::Get(key, Some(Arity::One("name"))));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let getall = Command::Hashes::<_, &str>(HashCommand::Get(key, Some(Arity::Many(vec!["name", "friend"]))));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let getall = Command::Hashes::<_, &str>(HashCommand::Len(key));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_hlen_no_exists() {
  let (key, url) = ("test_hlen_no_exists", get_redis_url());

  let result = async_std::task::block_on(async {
    let hlen = Command::Hashes::<_, &str>(HashCommand::Len(key));
    send(url.as_str(), hlen).await
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_hkeys_values() {
  let (key, url) = ("test_hkeys_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes::<_, &str>(HashCommand::Keys(key));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
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
    let hlen = Command::Hashes::<_, &str>(HashCommand::Keys(key));
    send(url.as_str(), hlen).await
  });

  assert_eq!(result.unwrap(), Response::Array(vec![]));
}

#[test]
fn test_mset_many() {
  let (one, two, url) = ("test_mset_many_1", "test_mset_many_2", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Strings::<_, &str>(StringCommand::Set(
      Arity::Many(vec![(one, "hello"), (two, "goodbye")]),
      None,
      Insertion::Always,
    ));
    send(url.as_str(), do_set).await?;
    let result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Get(Arity::Many(vec![one, two]))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(one))).await?;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(two))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("hello")),
      ResponseValue::String(String::from("goodbye"))
    ]),
  );
}

#[test]
fn test_msetnx_many() {
  let (one, two, url) = ("test_msetnx_many_1", "test_msetnx_many_2", get_redis_url());

  let result = async_std::task::block_on(async {
    let do_set = Command::Strings::<_, &str>(StringCommand::Set(
      Arity::Many(vec![(one, "hello"), (two, "goodbye")]),
      None,
      Insertion::IfNotExists,
    ));
    send(url.as_str(), do_set).await?;
    let result = send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Get(Arity::Many(vec![one, two]))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(one))).await?;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(two))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("hello")),
      ResponseValue::String(String::from("goodbye"))
    ]),
  );
}

#[test]
fn test_msetnx_already_exists() {
  let (one, two, url) = (
    "test_msetnx_alredy_exits_1",
    "test_msetnx_already_exists_2",
    get_redis_url(),
  );

  let result = async_std::task::block_on(async {
    send(
      url.as_str(),
      Command::Strings::<_, &str>(StringCommand::Set(Arity::One((one, "foo")), None, Insertion::Always)),
    )
    .await?;
    let do_set = Command::Strings::<_, &str>(StringCommand::Set(
      Arity::Many(vec![(one, "hello"), (two, "goodbye")]),
      None,
      Insertion::IfNotExists,
    ));
    let result = send(url.as_str(), do_set).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(one))).await?;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(two))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)),);
}

#[test]
fn test_hvals_values() {
  let (key, url) = ("test_hvals_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes::<_, &str>(HashCommand::Vals(key));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![ResponseValue::String(String::from("kramer"))])
  );
}

#[test]
fn test_hstrlen_values() {
  let (key, url) = ("test_hstrlen_values", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "name", "kramer")).await?;
    let getall = Command::Hashes::<_, &str>(HashCommand::StrLen(key, "name"));
    let result = send(url.as_str(), getall).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(6)));
}

#[test]
fn test_hincrby() {
  let (key, url) = ("test_hincrby", get_redis_url());

  let result = async_std::task::block_on(async {
    send(url.as_str(), set_field(key, "episodes", "10")).await?;
    let inc = Command::Hashes::<_, &str>(HashCommand::Incr(key, "episodes", 10));
    send(url.as_str(), inc).await?;
    let result = send(
      url.as_str(),
      Command::Hashes::<_, &str>(HashCommand::Get(key, Some(Arity::One("episodes")))),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    result
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("20")))
  );
}

#[test]
fn test_lrange() {
  let (key, url) = ("test_lrange", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Left, Insertion::Always),
      key,
      Arity::One("kramer"),
    ));
    send(url.as_str(), ins).await?;
    let out = send(url.as_str(), Command::List::<_, &str>(ListCommand::Range(key, 0, 10))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![ResponseValue::String(String::from("kramer"))])
  );
}

#[test]
fn test_lindex_present() {
  let (key, url) = ("test_lindex_present", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Left, Insertion::Always),
      key,
      Arity::One("kramer"),
    ));
    send(url.as_str(), ins).await?;
    let out = send(url.as_str(), Command::List::<_, &str>(ListCommand::Index(key, 0))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Item(ResponseValue::String(String::from("kramer")))
  );
}

#[test]
fn test_lindex_missing() {
  let (key, url) = ("test_lindex_missing", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(url.as_str(), Command::List::<_, &str>(ListCommand::Index(key, 0))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Empty));
}

#[test]
fn test_lrem_present() {
  let (key, url) = ("test_lrem_present", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Left, Insertion::Always),
      key,
      Arity::One("kramer"),
    ));
    send(url.as_str(), ins).await?;
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Rem(key, "kramer", 1)),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(1)));
}

#[test]
fn test_lrem_missing() {
  let (key, url) = ("test_lrem_missing", get_redis_url());

  let result = async_std::task::block_on(async {
    let out = send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Rem(key, "kramer", 1)),
    )
    .await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(result.unwrap(), Response::Item(ResponseValue::Integer(0)));
}

#[test]
fn test_ltrim_present() {
  let (key, url) = ("test_ltrim_present", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry", "elaine", "george"]),
    ));
    send(url.as_str(), ins).await?;
    send(url.as_str(), Command::List::<_, &str>(ListCommand::Trim(key, 0, 2))).await?;
    let out = send(url.as_str(), Command::List::<_, &str>(ListCommand::Range(key, 0, 10))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("kramer")),
      ResponseValue::String(String::from("jerry")),
      ResponseValue::String(String::from("elaine")),
    ])
  );
}

#[test]
fn test_linsert_left_present() {
  let (key, url) = ("test_linsert_left_present", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List::<_, &str>(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry", "elaine", "george"]),
    ));
    send(url.as_str(), ins).await?;
    send(
      url.as_str(),
      Command::List::<_, &str>(ListCommand::Insert(key, Side::Left, "george", "newman")),
    )
    .await?;
    let out = send(url.as_str(), Command::List::<_, &str>(ListCommand::Range(key, 0, 10))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("kramer")),
      ResponseValue::String(String::from("jerry")),
      ResponseValue::String(String::from("elaine")),
      ResponseValue::String(String::from("newman")),
      ResponseValue::String(String::from("george")),
    ])
  );
}

#[cfg(feature = "kramer-io")]
#[test]
fn test_linsert_right_present() {
  let (key, url) = ("test_linsert_right_present", get_redis_url());

  let result = async_std::task::block_on(async {
    let ins = Command::List(ListCommand::Push(
      (Side::Right, Insertion::Always),
      key,
      Arity::Many(vec!["kramer", "jerry", "elaine", "george"]),
    ));
    send(url.as_str(), ins).await?;
    send(
      url.as_str(),
      Command::List(ListCommand::Insert(key, Side::Right, "george", "newman")),
    )
    .await?;
    let out = send(url.as_str(), Command::List(ListCommand::Range(key, 0, 10))).await;
    send(url.as_str(), Command::Del::<_, &str>(Arity::One(key))).await?;
    out
  });

  assert_eq!(
    result.unwrap(),
    Response::Array(vec![
      ResponseValue::String(String::from("kramer")),
      ResponseValue::String(String::from("jerry")),
      ResponseValue::String(String::from("elaine")),
      ResponseValue::String(String::from("george")),
      ResponseValue::String(String::from("newman")),
    ])
  );
}
