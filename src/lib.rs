//! An implementation of the [redis protocol specification][redis] with an execution helper using
//! the [`TcpStream`][tcp-stream] provided by [async-std].
//!
//! ## Example
//!
//! ```
//! use kramer::{Command, send};
//! use std::env::{var};
//!
//! fn get_redis_url() -> String {
//!   let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
//!   let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
//!   format!("{}:{}", host, port)
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   let url = get_redis_url();
//!   let cmd = Command::Keys("*");
//!
//!   async_std::task::block_on(async {
//!     match send(url.as_str(), cmd).await {
//!       Ok(keys) => println!("keys is a response: {:?}", keys),
//!       Err(e) => println!("unable to send: {:?}", e),
//!     }
//!   });
//!
//!   Ok(())
//! }
//! ```
//!
//! [redis]: https://redis.io/topics/protocol
//! [async-std]: https://github.com/async-rs/async-std
//! [tcp-stream]: https://docs.rs/async-std/0.99.11/async_std/net/struct.TcpStream.html
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
  Pop(Side, S, Option<(Option<Arity<S>>, u64)>),
  Rem(S, S, u64),
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
      ListCommand::Range(key, from, to) => {
        let end = format!("{}{}", format_bulk_string(from), format_bulk_string(to));
        write!(formatter, "*2\r\n$6\r\nLRANGE\r\n{}{}", format_bulk_string(key), end)
      }
      ListCommand::Len(key) => write!(formatter, "*2\r\n$4\r\nLLEN\r\n{}", format_bulk_string(key)),
      ListCommand::Pop(side, key, block) => {
        let (cmd, ext, kc) = match (side, block) {
          (Side::Left, None) => ("LPOP", format!(""), 0),
          (Side::Right, None) => ("RPOP", format!(""), 0),
          (Side::Left, Some((None, timeout))) => ("BLPOP", format_bulk_string(timeout), 1),
          (Side::Right, Some((None, timeout))) => ("BRPOP", format_bulk_string(timeout), 1),
          (Side::Left, Some((Some(values), timeout))) => {
            let (vc, ext) = match values {
              Arity::One(value) => (1, format_bulk_string(value)),
              Arity::Many(values) => (values.len(), values.iter().map(format_bulk_string).collect::<String>()),
            };
            ("BLPOP", format!("{}{}", ext, format_bulk_string(timeout)), vc + 1)
          }
          (Side::Right, Some((Some(values), timeout))) => {
            let (vc, ext) = match values {
              Arity::One(value) => (1, format_bulk_string(value)),
              Arity::Many(values) => (values.len(), values.iter().map(format_bulk_string).collect::<String>()),
            };
            ("BRPOP", format!("{}{}", ext, format_bulk_string(timeout)), vc + 1)
          }
        };
        write!(
          formatter,
          "*{}\r\n${}\r\n{}\r\n{}{}",
          2 + kc,
          cmd.len(),
          cmd,
          format_bulk_string(key),
          ext
        )
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
  Get(S),
  Decr(S),
  Append(S, S),
}

impl<S: std::fmt::Display> std::fmt::Display for StringCommand<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      StringCommand::Decr(key) => write!(formatter, "*2\r\n$4\r\nDECR\r\n{}", format_bulk_string(key)),
      StringCommand::Get(key) => write!(formatter, "*2\r\n$3\r\nGET\r\n{}", format_bulk_string(key)),
      StringCommand::Append(key, value) => write!(
        formatter,
        "*3\r\n$6\r\nAPPEND\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(value)
      ),
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

pub async fn execute<C, S>(mut connection: C, message: Command<S>) -> Result<Response, IOError>
where
  S: std::fmt::Display,
  C: async_std::io::Write + async_std::io::Read + std::marker::Unpin,
{
  write!(connection, "{}", message).await;

  let mut lines = async_std::io::BufReader::new(connection).lines();

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

pub async fn send<S>(addr: &str, message: Command<S>) -> Result<Response, IOError>
where
  S: std::fmt::Display,
{
  let mut stream = TcpStream::connect(addr).await?;
  execute(&mut stream, message).await
}

#[cfg(test)]
mod fmt_tests {
  use super::{Arity, Command, Insertion, ListCommand, Side, StringCommand};
  use std::io::Write;

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
      format!("{}", Command::List(ListCommand::Pop(Side::Right, "seinfeld", None))),
      "*2\r\n$4\r\nRPOP\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_command_lpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Pop(Side::Left, "seinfeld", None))),
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
  fn test_command_brpop_timeout_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(Side::Right, "seinfeld", Some((None, 10))))
      ),
      "*3\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_command_brpop_timeout_multi_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(
          Side::Right,
          "seinfeld",
          Some((Some(Arity::One("derry-girls")), 10))
        ))
      ),
      "*4\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_command_brpop_timeout_multi_many_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(
          Side::Right,
          "seinfeld",
          Some((Some(Arity::Many(vec!["derry-girls", "creek"])), 10))
        ))
      ),
      "*5\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$5\r\ncreek\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_command_blpop_timeout_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(Side::Left, "seinfeld", Some((None, 10))))
      ),
      "*3\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_command_blpop_timeout_multi_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(
          Side::Left,
          "seinfeld",
          Some((Some(Arity::One("derry-girls")), 10))
        ))
      ),
      "*4\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_command_blpop_timeout_multi_many_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List(ListCommand::Pop(
          Side::Left,
          "seinfeld",
          Some((Some(Arity::Many(vec!["derry-girls", "creek"])), 10))
        ))
      ),
      "*5\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$5\r\ncreek\r\n$2\r\n10\r\n"
    );
  }

  /*

  #[test]
  fn test_command_brpop_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::BlockPop(Side::Right, "seinfeld", 1))),
      "*3\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$1\r\n1\r\n"
    );
  }
  */

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
  fn test_command_set_fmt_if_not_exists() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings(StringCommand::Set("seinfeld", "kramer", None, Insertion::IfNotExists))
      ),
      "*4\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nNX\r\n"
    );
  }

  #[test]
  fn test_command_set_fmt_if_exists() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings(StringCommand::Set("seinfeld", "kramer", None, Insertion::IfExists))
      ),
      "*4\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nXX\r\n"
    );
  }

  #[test]
  fn test_command_lrem_fmt() {
    assert_eq!(
      format!("{}", Command::List(ListCommand::Rem("seinfeld", "kramer", 1))),
      "*4\r\n$4\r\nLREM\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$1\r\n1\r\n"
    );
  }

  #[test]
  fn test_command_get_fmt() {
    assert_eq!(
      format!("{}", Command::Strings(StringCommand::Get("seinfeld"))),
      "*2\r\n$3\r\nGET\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_command_decr_fmt() {
    assert_eq!(
      format!("{}", Command::Strings(StringCommand::Decr("seinfeld"))),
      "*2\r\n$4\r\nDECR\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_command_append_fmt() {
    assert_eq!(
      format!("{}", Command::Strings(StringCommand::Append("seinfeld", "kramer"))),
      "*3\r\n$6\r\nAPPEND\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_macro_write() {
    let cmd = Command::Strings(StringCommand::Decr("one"));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*2\r\n$4\r\nDECR\r\n$3\r\none\r\n")
    );
  }
}
