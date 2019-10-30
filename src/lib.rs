extern crate async_std;

use async_std::net::TcpStream;
use async_std::prelude::*;
use std::io::{Error as IOError, ErrorKind as IOErrorKind};

#[derive(Debug)]
enum ResponseLine {
  Array(usize),
  SimpleString(String),
  Error(String),
  Integer(i64),
  BulkString(usize),
  Null,
}

#[derive(Debug, PartialEq)]
pub enum ResponseValue {
  Empty,
  String(String),
  Integer(i64),
}

#[derive(Debug, PartialEq)]
pub enum Response {
  Array(Vec<ResponseValue>),
  Item(ResponseValue),
  Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Side {
  Left,
  Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Insertion {
  Always,
  IfExists,
  IfNotExists,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arity<S> {
  Many(Vec<S>),
  One(S),
}

#[derive(Debug)]
pub enum ListCommand<S>
where
  S: std::fmt::Display,
{
  Len(S),
  Push((Side, Insertion), S, Arity<S>),
  Pop(Side, S),
  Rem(S, S, u64),
  BlockPop(Side, S, u64),
  Range(S, i64, i64),
}

fn format_bulk_string<S: std::fmt::Display>(input: S) -> String {
  let as_str = format!("{}", input);
  format!("${}\r\n{}\r\n", as_str.len(), as_str)
}

impl<S: std::fmt::Display> std::fmt::Display for ListCommand<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      ListCommand::Rem(key, value, count) => {
        let end = format!(
          "{}{}{}",
          format_bulk_string(key),
          format_bulk_string(value),
          format_bulk_string(count)
        );

        write!(formatter, "*4\r\n$4\r\nLREM\r\n{}", end)
      }
      ListCommand::BlockPop(side, key, time) => {
        let end = format!("{}{}", format_bulk_string(key), format_bulk_string(time));
        let cmd = match side {
          Side::Left => "BLPOP",
          Side::Right => "BRPOP",
        };
        write!(formatter, "*3\r\n$5\r\n{}\r\n{}", cmd, end)
      }
      ListCommand::Range(key, from, to) => {
        let end = format!("{}{}", format_bulk_string(from), format_bulk_string(to));
        write!(formatter, "*2\r\n$6\r\nLRANGE\r\n{}{}", format_bulk_string(key), end)
      }
      ListCommand::Len(key) => write!(formatter, "*2\r\n$4\r\nLLEN\r\n{}", format_bulk_string(key)),
      ListCommand::Pop(side, key) => {
        let cmd = match side {
          Side::Left => "LPOP",
          Side::Right => "RPOP",
        };
        write!(formatter, "*2\r\n$4\r\n{}\r\n{}", cmd, format_bulk_string(key))
      }
      ListCommand::Push(operation, k, Arity::One(v)) => {
        let cmd = match operation {
          (Side::Left, Insertion::IfExists) => "LPUSHX",
          (Side::Right, Insertion::IfExists) => "RPUSHX",
          (Side::Left, _) => "LPUSH",
          (Side::Right, _) => "RPUSH",
        };
        let parts = format!("{}{}", format_bulk_string(k), format_bulk_string(v),);
        write!(formatter, "*3\r\n${}\r\n{}\r\n{}", cmd.len(), cmd, parts)
      }
      ListCommand::Push(operation, k, Arity::Many(v)) => {
        let size = v.len();
        let cmd = match operation {
          (Side::Left, Insertion::IfExists) => "LPUSHX",
          (Side::Right, Insertion::IfExists) => "RPUSHX",
          (Side::Left, _) => "LPUSH",
          (Side::Right, _) => "RPUSH",
        };
        let parts = format!(
          "{}{}",
          format_bulk_string(k),
          v.iter().map(format_bulk_string).collect::<String>()
        );
        write!(formatter, "*{}\r\n${}\r\n{}\r\n{}", 2 + size, cmd.len(), cmd, parts)
      }
    }
  }
}

#[derive(Debug)]
pub enum StringCommand<S>
where
  S: std::fmt::Display,
{
  Set(S, S, Option<std::time::Duration>, Insertion),
}

impl<S: std::fmt::Display> std::fmt::Display for StringCommand<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      StringCommand::Set(key, value, timeout, insertion) => {
        let (k, v) = (format_bulk_string(key), format_bulk_string(value));
        let (cx, px) = match timeout {
          None => (0, format!("")),
          Some(t) => (
            2,
            format!("{}{}", format_bulk_string("PX"), format_bulk_string(t.as_millis())),
          ),
        };
        let (ci, i) = match insertion {
          Insertion::IfExists => (1, format_bulk_string("XX")),
          Insertion::IfNotExists => (1, format_bulk_string("NX")),
          Insertion::Always => (0, format!("")),
        };
        write!(formatter, "*{}\r\n$3\r\nSET\r\n{}{}{}{}", 3 + ci + cx, k, v, px, i)
      }
    }
  }
}

#[derive(Debug)]
pub enum Command<S>
where
  S: std::fmt::Display,
{
  Keys(S),
  Del(Arity<S>),
  Exists(Arity<S>),
  List(ListCommand<S>),
  Strings(StringCommand<S>),
}

impl<S: std::fmt::Display> std::fmt::Display for Command<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Command::Keys(value) => write!(formatter, "*2\r\n$4\r\nKEYS\r\n{}", format_bulk_string(value)),
      Command::Exists(Arity::Many(values)) => {
        let len = values.len();
        let right = values.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$6\r\nEXISTS\r\n{}", len + 1, right)
      }
      Command::Exists(Arity::One(value)) => write!(formatter, "*2\r\n$6\r\nEXISTS\r\n{}", format_bulk_string(value)),
      Command::Del(Arity::One(value)) => write!(formatter, "*2\r\n$3\r\nDEL\r\n{}", format_bulk_string(value)),
      Command::Del(Arity::Many(values)) => {
        let len = values.len();
        let right = values.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$3\r\nDEL\r\n{}", len + 1, right)
      }
      Command::List(list_command) => write!(formatter, "{}", list_command),
      Command::Strings(string_command) => write!(formatter, "{}", string_command),
    }
  }
}

fn read_line_size(line: String) -> Result<Option<usize>, IOError> {
  match line.split_at(1).1 {
    "-1" => Ok(None),
    value => value
      .parse::<usize>()
      .map_err(|e| {
        IOError::new(
          IOErrorKind::Other,
          format!("invalid array length value '{}': {}", line.as_str(), e),
        )
      })
      .map(|v| Some(v)),
  }
}

fn readline(result: Option<Result<String, IOError>>) -> Result<ResponseLine, IOError> {
  let line = result.ok_or_else(|| IOError::new(IOErrorKind::Other, "no line to work with"))??;

  match line.bytes().next() {
    Some(b'*') => match read_line_size(line)? {
      None => Ok(ResponseLine::Null),
      Some(size) => Ok(ResponseLine::Array(size)),
    },
    Some(b'$') => match read_line_size(line)? {
      Some(size) => Ok(ResponseLine::BulkString(size)),
      None => Ok(ResponseLine::Null),
    },
    Some(b'-') => Ok(ResponseLine::Error(line)),
    Some(b'+') => Ok(ResponseLine::SimpleString(String::from(line.split_at(1).1))),
    Some(b':') => {
      let (_, rest) = line.split_at(1);
      rest
        .parse::<i64>()
        .map_err(|e| IOError::new(IOErrorKind::Other, format!("{:?}", e)))
        .and_then(|v| Ok(ResponseLine::Integer(v)))
    }
    Some(unknown) => Err(IOError::new(
      IOErrorKind::Other,
      format!("invalid message byte leader: {}", unknown),
    )),
    None => Err(IOError::new(
      IOErrorKind::Other,
      "empty line in response, unable to determine type",
    )),
  }
}

pub async fn send<S: std::fmt::Display>(addr: &str, message: Command<S>) -> Result<Response, IOError> {
  let mut stream = TcpStream::connect(addr).await?;
  stream.write_all(format!("{}", message).as_bytes()).await?;

  let mut lines = async_std::io::BufReader::new(stream).lines();

  match readline(lines.next().await) {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while let Ok(kind) = readline(lines.next().await) {
        match kind {
          ResponseLine::BulkString(size) => match lines.next().await {
            Some(Ok(bulky)) if bulky.len() == size => {
              store.push(ResponseValue::String(bulky));
            }
            _ => break,
          },
          _ => break,
        }

        if store.len() >= size {
          return Ok(Response::Array(store));
        }
      }

      Ok(Response::Array(store))
    }
    Ok(ResponseLine::BulkString(size)) => {
      if size < 1 {
        return Ok(Response::Item(ResponseValue::Empty));
      }

      let out = lines
        .next()
        .await
        .ok_or_else(|| IOError::new(IOErrorKind::Other, "no line to work with"))??;

      Ok(Response::Item(ResponseValue::String(out)))
    }
    Ok(ResponseLine::Null) => Ok(Response::Item(ResponseValue::Empty)),
    Ok(ResponseLine::SimpleString(simple)) => Ok(Response::Item(ResponseValue::String(simple))),
    Ok(ResponseLine::Integer(value)) => Ok(Response::Item(ResponseValue::Integer(value))),
    Ok(ResponseLine::Error(e)) => Err(IOError::new(IOErrorKind::Other, e)),
    Err(e) => Err(e),
  }
}

#[cfg(test)]
mod fmt_tests {
  use super::{Arity, Command, Insertion, ListCommand, Side, StringCommand};

  #[test]
  fn test_command_keys_fmt() {
    assert_eq!(
      format!("{}", Command::Keys(String::from("*"))),
      "*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n"
    );
  }

  #[test]
  fn test_command_llen_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Len("kramer"))),
      "*2\r\n$4\r\nLLEN\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_lpush_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Push(
          (Side::Left, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$5\r\nLPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_rpush_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Push(
          (Side::Right, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$5\r\nRPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_rpushx_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Push(
          (Side::Right, Insertion::IfExists),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$6\r\nRPUSHX\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_lpushx_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Push(
          (Side::Left, Insertion::IfExists),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$6\r\nLPUSHX\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_rpush_fmt_multi() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Push(
          (Side::Right, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer", "jerry"]),
        ))
      ),
      "*4\r\n$5\r\nRPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$5\r\njerry\r\n"
    );
  }

  #[test]
  fn test_command_rpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Pop(Side::Right, "seinfeld"))),
      "*2\r\n$4\r\nRPOP\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_command_lpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Pop(Side::Left, "seinfeld"))),
      "*2\r\n$4\r\nLPOP\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_command_lrange_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Range("seinfeld", 0, -1))),
      "*2\r\n$6\r\nLRANGE\r\n$8\r\nseinfeld\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
  }

  #[test]
  fn test_command_blpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::BlockPop(Side::Left, "seinfeld", 1))),
      "*3\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$1\r\n1\r\n"
    );
  }

  #[test]
  fn test_command_brpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::BlockPop(Side::Right, "seinfeld", 1))),
      "*3\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$1\r\n1\r\n"
    );
  }

  #[test]
  fn test_command_del_fmt() {
    assert_eq!(
      format!("{}", Command::Del(Arity::Many(vec!["kramer"]))),
      "*2\r\n$3\r\nDEL\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_del_fmt_multi() {
    assert_eq!(
      format!("{}", Command::Del(Arity::Many(vec!["kramer", "jerry"]))),
      "*3\r\n$3\r\nDEL\r\n$6\r\nkramer\r\n$5\r\njerry\r\n"
    );
  }

  #[test]
  fn test_command_set_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings(StringCommand::Set("seinfeld", "kramer", None, Insertion::Always))
      ),
      "*3\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_command_set_fmt_duration() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings(StringCommand::Set(
          "seinfeld",
          "kramer",
          Some(std::time::Duration::new(1, 0)),
          Insertion::Always
        ))
      ),
      "*5\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nPX\r\n$4\r\n1000\r\n"
    );
  }

  #[test]
  fn test_command_lrem_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Rem("seinfeld", "kramer", 1))),
      "*4\r\n$4\r\nLREM\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$1\r\n1\r\n"
    );
  }
}

#[cfg(test)]
mod send_tests {
  use super::{send, Arity, Command, Insertion, ListCommand, Response, ResponseValue, Side, StringCommand};
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
}
