//! An implementation of the [redis protocol specification][redis] with an execution helper using
//! the [`TcpStream`][tcp-stream] provided by [async-std].
//!
//! ## Example
//!
//! ```
//! use kramer::{Command, StringCommand, Arity, Insertion};
//! use std::env::{var};
//! use std::io::prelude::*;
//!
//! fn get_redis_url() -> String {
//!   let host = var("REDIS_HOST").unwrap_or(String::from("0.0.0.0"));
//!   let port = var("REDIS_PORT").unwrap_or(String::from("6379"));
//!   format!("{}:{}", host, port)
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!   let url = get_redis_url();
//!   let cmd = Command::Keys::<_, &str>("*");
//!   let mut stream = std::net::TcpStream::connect(url)?;
//!   write!(stream, "{}", cmd)?;
//!   write!(stream, "{}", StringCommand::Set(Arity::One(("name", "kramer")), None, Insertion::Always))?;
//!   Ok(())
//! }
//! ```
//!
//! [redis]: https://redis.io/topics/protocol
//! [async-std]: https://github.com/async-rs/async-std
//! [tcp-stream]: https://docs.rs/async-std/0.99.11/async_std/net/struct.TcpStream.html
mod response;
pub use response::{Response, ResponseLine, ResponseValue};

#[cfg(feature = "kramer-async")]
mod async_io;
#[cfg(feature = "kramer-async")]
pub use async_io::{execute, read, send};

#[cfg(not(feature = "kramer-async"))]
mod sync_io;
#[cfg(not(feature = "kramer-async"))]
pub use sync_io::{execute, read, send};

mod modifiers;
use modifiers::format_bulk_string;
pub use modifiers::{Arity, Insertion, Side};

mod lists;
pub use lists::ListCommand;

mod sets;
pub use sets::SetCommand;

mod strings;
pub use strings::StringCommand;

mod hashes;
pub use hashes::HashCommand;

#[derive(Debug)]
pub enum Command<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  Keys(S),
  Del(Arity<S>),
  Exists(Arity<S>),
  List(ListCommand<S>),
  Strings(StringCommand<S>),
  Hashes(HashCommand<S, V>),
  Sets(SetCommand<S, V>),
  Echo(S),
}

impl<S, V> std::fmt::Display for Command<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Command::Echo(value) => write!(formatter, "*2\r\n$4\r\nECHO\r\n{}", format_bulk_string(value)),
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
      Command::Hashes(hash_command) => write!(formatter, "{}", hash_command),
      Command::Sets(set_command) => write!(formatter, "{}", set_command),
    }
  }
}

#[cfg(test)]
mod fmt_tests {
  use super::{Arity, Command, HashCommand, Insertion, ListCommand, Side, StringCommand};
  use std::io::Write;

  #[test]
  fn test_keys_fmt() {
    assert_eq!(
      format!("{}", Command::Keys::<&str, &str>("*")),
      "*2\r\n$4\r\nKEYS\r\n$1\r\n*\r\n"
    );
  }

  #[test]
  fn test_llen_fmt() {
    assert_eq!(
      format!("{}", Command::List::<&str, &str>(ListCommand::Len("kramer"))),
      "*2\r\n$4\r\nLLEN\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_lpush_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Push(
          (Side::Left, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$5\r\nLPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_rpush_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Push(
          (Side::Right, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$5\r\nRPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_rpushx_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Push(
          (Side::Right, Insertion::IfExists),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$6\r\nRPUSHX\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_lpushx_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Push(
          (Side::Left, Insertion::IfExists),
          "seinfeld",
          Arity::Many(vec!["kramer"]),
        ))
      ),
      "*3\r\n$6\r\nLPUSHX\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_rpush_fmt_multi() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Push(
          (Side::Right, Insertion::Always),
          "seinfeld",
          Arity::Many(vec!["kramer", "jerry"]),
        ))
      ),
      "*4\r\n$5\r\nRPUSH\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$5\r\njerry\r\n"
    );
  }

  #[test]
  fn test_rpop_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(Side::Right, "seinfeld", None))
      ),
      "*2\r\n$4\r\nRPOP\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_lpop_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(Side::Left, "seinfeld", None))
      ),
      "*2\r\n$4\r\nLPOP\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_lrange_fmt() {
    assert_eq!(
      format!("{}", Command::List::<&str, &str>(ListCommand::Range("seinfeld", 0, -1))),
      "*4\r\n$6\r\nLRANGE\r\n$8\r\nseinfeld\r\n$1\r\n0\r\n$2\r\n-1\r\n"
    );
  }

  #[test]
  fn test_brpop_timeout_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(Side::Right, "seinfeld", Some((None, 10))))
      ),
      "*3\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_brpop_timeout_multi_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(
          Side::Right,
          "seinfeld",
          Some((Some(Arity::One("derry-girls")), 10))
        ))
      ),
      "*4\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_brpop_timeout_multi_many_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(
          Side::Right,
          "seinfeld",
          Some((Some(Arity::Many(vec!["derry-girls", "creek"])), 10))
        ))
      ),
      "*5\r\n$5\r\nBRPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$5\r\ncreek\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_blpop_timeout_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(Side::Left, "seinfeld", Some((None, 10))))
      ),
      "*3\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_blpop_timeout_multi_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(
          Side::Left,
          "seinfeld",
          Some((Some(Arity::One("derry-girls")), 10))
        ))
      ),
      "*4\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_blpop_timeout_multi_many_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Pop(
          Side::Left,
          "seinfeld",
          Some((Some(Arity::Many(vec!["derry-girls", "creek"])), 10))
        ))
      ),
      "*5\r\n$5\r\nBLPOP\r\n$8\r\nseinfeld\r\n$11\r\nderry-girls\r\n$5\r\ncreek\r\n$2\r\n10\r\n"
    );
  }

  #[test]
  fn test_del_fmt() {
    assert_eq!(
      format!("{}", Command::Del::<&str, &str>(Arity::Many(vec!["kramer"]))),
      "*2\r\n$3\r\nDEL\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_del_fmt_multi() {
    assert_eq!(
      format!("{}", Command::Del::<&str, &str>(Arity::Many(vec!["kramer", "jerry"]))),
      "*3\r\n$3\r\nDEL\r\n$6\r\nkramer\r\n$5\r\njerry\r\n"
    );
  }

  #[test]
  fn test_set_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings::<&str, &str>(StringCommand::Set(
          Arity::One(("seinfeld", "kramer")),
          None,
          Insertion::Always
        ))
      ),
      "*3\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_set_fmt_duration() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings::<&str, &str>(StringCommand::Set(
          Arity::One(("seinfeld", "kramer")),
          Some(std::time::Duration::new(1, 0)),
          Insertion::Always
        ))
      ),
      "*5\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nPX\r\n$4\r\n1000\r\n"
    );
  }

  #[test]
  fn test_set_fmt_if_not_exists() {
    let cmd = Command::Strings::<&str, &str>(StringCommand::Set(
      Arity::One(("seinfeld", "kramer")),
      None,
      Insertion::IfNotExists,
    ));
    assert_eq!(
      format!("{}", cmd),
      "*4\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nNX\r\n"
    );
  }

  #[test]
  fn test_set_fmt_if_exists() {
    let cmd = Command::Strings::<&str, &str>(StringCommand::Set(
      Arity::One(("seinfeld", "kramer")),
      None,
      Insertion::IfExists,
    ));
    assert_eq!(
      format!("{}", cmd),
      "*4\r\n$3\r\nSET\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$2\r\nXX\r\n"
    );
  }

  #[test]
  fn test_lrem_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::List::<&str, &str>(ListCommand::Rem("seinfeld", "kramer", 1))
      ),
      "*4\r\n$4\r\nLREM\r\n$8\r\nseinfeld\r\n$1\r\n1\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_get_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings::<&str, &str>(StringCommand::Get(Arity::One("seinfeld")))
      ),
      "*2\r\n$3\r\nGET\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_decr_fmt() {
    assert_eq!(
      format!("{}", Command::Strings::<&str, &str>(StringCommand::Decr("seinfeld", 1))),
      "*2\r\n$4\r\nDECR\r\n$8\r\nseinfeld\r\n"
    );
  }

  #[test]
  fn test_append_fmt() {
    assert_eq!(
      format!(
        "{}",
        Command::Strings::<&str, &str>(StringCommand::Append("seinfeld", "kramer"))
      ),
      "*3\r\n$6\r\nAPPEND\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n"
    );
  }

  #[test]
  fn test_macro_write() {
    let cmd = Command::Strings::<&str, &str>(StringCommand::Decr("one", 1));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*2\r\n$4\r\nDECR\r\n$3\r\none\r\n")
    );
  }

  #[test]
  fn test_hdel_single() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Del("seinfeld", Arity::One("kramer")));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*3\r\n$4\r\nHDEL\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n")
    );
  }

  #[test]
  fn test_hdel_many() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Del("seinfeld", Arity::Many(vec!["kramer", "jerry"])));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$4\r\nHDEL\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n$5\r\njerry\r\n")
    );
  }

  #[test]
  fn test_hset_single() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Set(
      "seinfeld",
      Arity::One(("name", "kramer")),
      Insertion::Always,
    ));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$4\r\nHSET\r\n$8\r\nseinfeld\r\n$4\r\nname\r\n$6\r\nkramer\r\n")
    );
  }

  #[test]
  fn test_hexists() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Exists("seinfeld", "kramer"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$7\r\nHEXISTS\r\n$8\r\nseinfeld\r\n$6\r\nkramer\r\n")
    );
  }

  #[test]
  fn test_echo() {
    let cmd = Command::Echo::<&str, &str>("hello");
    assert_eq!(format!("{}", cmd), String::from("*2\r\n$4\r\nECHO\r\n$5\r\nhello\r\n"));
  }

  #[test]
  fn test_hset_many() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Set(
      "seinfeld",
      Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
      Insertion::Always,
    ));
    assert_eq!(
      format!("{}", cmd),
      String::from(
        "*6\r\n$4\r\nHSET\r\n$8\r\nseinfeld\r\n$4\r\nname\r\n$6\r\nkramer\r\n$6\r\nfriend\r\n$5\r\njerry\r\n"
      )
    );
  }

  #[test]
  fn test_hgetall() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Get("seinfeld", None));
    assert_eq!(
      format!("{}", cmd),
      String::from("*2\r\n$7\r\nHGETALL\r\n$8\r\nseinfeld\r\n")
    );
  }

  #[test]
  fn test_mset() {
    let cmd = Command::Strings::<&str, &str>(StringCommand::Set(
      Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
      None,
      Insertion::Always,
    ));
    assert_eq!(
      format!("{}", cmd),
      String::from("*5\r\n$4\r\nMSET\r\n$4\r\nname\r\n$6\r\nkramer\r\n$6\r\nfriend\r\n$5\r\njerry\r\n")
    );
  }

  #[test]
  fn test_msetnx() {
    let cmd = Command::Strings::<&str, &str>(StringCommand::Set(
      Arity::Many(vec![("name", "kramer"), ("friend", "jerry")]),
      None,
      Insertion::IfNotExists,
    ));
    assert_eq!(
      format!("{}", cmd),
      String::from("*5\r\n$6\r\nMSETNX\r\n$4\r\nname\r\n$6\r\nkramer\r\n$6\r\nfriend\r\n$5\r\njerry\r\n")
    );
  }

  #[test]
  fn test_hincrby() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Incr("kramer", "episodes", 10));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$7\r\nHINCRBY\r\n$6\r\nkramer\r\n$8\r\nepisodes\r\n$2\r\n10\r\n")
    );
  }

  #[test]
  fn test_hlen() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Len("seinfeld"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*2\r\n$4\r\nHLEN\r\n$8\r\nseinfeld\r\n")
    );
  }

  #[test]
  fn test_hvals() {
    let cmd = Command::Hashes::<&str, &str>(HashCommand::Vals("seinfeld"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*2\r\n$5\r\nHVALS\r\n$8\r\nseinfeld\r\n")
    );
  }

  #[test]
  fn test_hstrlen() {
    let cmd = Command::Hashes::<_, &str>(HashCommand::StrLen("seinfeld", "name"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$7\r\nHSTRLEN\r\n$8\r\nseinfeld\r\n$4\r\nname\r\n")
    );
  }

  #[test]
  fn test_hget() {
    let cmd = Command::Hashes::<_, &str>(HashCommand::Get("seinfeld", Some(Arity::One("name"))));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$4\r\nHGET\r\n$8\r\nseinfeld\r\n$4\r\nname\r\n")
    );
  }

  #[test]
  fn test_ltrim() {
    let cmd = Command::List::<_, &str>(ListCommand::Trim("episodes", 0, 10));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$5\r\nLTRIM\r\n$8\r\nepisodes\r\n$1\r\n0\r\n$2\r\n10\r\n")
    );
  }

  #[test]
  fn test_linsert_before() {
    let cmd = Command::List::<_, &str>(ListCommand::Insert("episodes", Side::Left, "10", "9"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*5\r\n$7\r\nLINSERT\r\n$8\r\nepisodes\r\n$6\r\nBEFORE\r\n$2\r\n10\r\n$1\r\n9\r\n")
    );
  }

  #[test]
  fn test_linsert_after() {
    let cmd = Command::List::<_, &str>(ListCommand::Insert("episodes", Side::Right, "10", "11"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*5\r\n$7\r\nLINSERT\r\n$8\r\nepisodes\r\n$5\r\nAFTER\r\n$2\r\n10\r\n$2\r\n11\r\n")
    );
  }

  #[test]
  fn test_lrem() {
    let cmd = Command::List::<_, &str>(ListCommand::Rem("episodes", "10", 100));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$4\r\nLREM\r\n$8\r\nepisodes\r\n$3\r\n100\r\n$2\r\n10\r\n")
    );
  }

  #[test]
  fn test_lindex() {
    let cmd = Command::List::<_, &str>(ListCommand::Index("episodes", 1));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$6\r\nLINDEX\r\n$8\r\nepisodes\r\n$1\r\n1\r\n")
    );
  }

  #[test]
  fn test_lset() {
    let cmd = Command::List::<_, &str>(ListCommand::Set("episodes", 1, "pilot"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$4\r\nLSET\r\n$8\r\nepisodes\r\n$1\r\n1\r\n$5\r\npilot\r\n")
    );
  }
}
