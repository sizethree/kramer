use crate::modifiers::{format_bulk_string, Arity};

#[derive(Debug)]
pub enum SetCommand<S>
where
  S: std::fmt::Display,
{
  Add(S, Arity<S>),
  Members(S),
}

impl<S: std::fmt::Display> std::fmt::Display for SetCommand<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    match self {
      SetCommand::Add(key, Arity::One(member)) => write!(
        formatter,
        "*3\r\n$4\r\nSADD\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(member)
      ),
      SetCommand::Add(key, Arity::Many(members)) => {
        let count = members.len();
        let tail = members.iter().map(format_bulk_string).collect::<String>();
        write!(
          formatter,
          "*{}\r\n$4\r\nSADD\r\n{}{}",
          count + 2,
          format_bulk_string(key),
          tail
        )
      }
      SetCommand::Members(key) => write!(formatter, "*2\r\n$8\r\nSMEMBERS\r\n{}", format_bulk_string(key)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::SetCommand;
  use crate::modifiers::Arity;
  use std::io::prelude::*;

  #[test]
  fn test_sadd_single() {
    let cmd = SetCommand::Add("seasons", Arity::One("one"));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*3\r\n$4\r\nSADD\r\n$7\r\nseasons\r\n$3\r\none\r\n")
    );
  }

  #[test]
  fn test_sadd_multi() {
    let cmd = SetCommand::Add("seasons", Arity::Many(vec!["one", "two"]));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*4\r\n$4\r\nSADD\r\n$7\r\nseasons\r\n$3\r\none\r\n$3\r\ntwo\r\n")
    );
  }

  #[test]
  fn test_smembers_multi() {
    let cmd = SetCommand::Members("seasons");
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*2\r\n$8\r\nSMEMBERS\r\n$7\r\nseasons\r\n")
    );
  }
}
