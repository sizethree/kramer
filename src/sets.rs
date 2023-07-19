use crate::modifiers::{format_bulk_string, Arity};

/// The `SetCommand` is used for working with redis keys that are sets: unique collections
/// of values.
#[derive(Debug)]
pub enum SetCommand<S, V> {
  /// Adds a member(s) to a set.
  Add(S, Arity<V>),

  /// Removes a member(s) to a set.
  Rem(S, Arity<V>),

  /// Returns the amount of members in the set.
  Card(S),

  /// Returns the members of the set resulting from the union of all the given sets.
  Union(Arity<S>),

  /// Returns the members of the set resulting from the intersection of all the given sets.
  Inter(Arity<S>),

  /// Returns whether or not the given value is a member of the set.
  IsMember(S, V),

  /// Returns the members of the set resulting from the difference of all the given sets.
  Diff(Arity<S>),

  /// Returns the members of the set.
  Members(S),

  /// Removes elements from the set.
  Pop(S, u64),
}

impl<S, V> std::fmt::Display for SetCommand<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    match self {
      SetCommand::Card(key) => write!(formatter, "*2\r\n$5\r\nSCARD\r\n{}", format_bulk_string(key)),
      SetCommand::IsMember(key, value) => write!(
        formatter,
        "*3\r\n$9\r\nSISMEMBER\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(value)
      ),

      SetCommand::Inter(Arity::One(member)) => {
        write!(formatter, "*2\r\n$6\r\nSINTER\r\n{}", format_bulk_string(member))
      }
      SetCommand::Inter(Arity::Many(members)) => {
        let count = members.len();
        let tail = members.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$6\r\nSINTER\r\n{}", count + 1, tail)
      }

      SetCommand::Diff(Arity::One(member)) => write!(formatter, "*2\r\n$5\r\nSDIFF\r\n{}", format_bulk_string(member)),
      SetCommand::Diff(Arity::Many(members)) => {
        let count = members.len();
        let tail = members.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$5\r\nSDIFF\r\n{}", count + 1, tail)
      }

      SetCommand::Union(Arity::One(member)) => {
        write!(formatter, "*2\r\n$6\r\nSUNION\r\n{}", format_bulk_string(member))
      }
      SetCommand::Union(Arity::Many(members)) => {
        let count = members.len();
        let tail = members.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$6\r\nSUNION\r\n{}", count + 1, tail)
      }

      SetCommand::Rem(key, Arity::One(member)) => write!(
        formatter,
        "*3\r\n$4\r\nSREM\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(member)
      ),
      SetCommand::Rem(key, Arity::Many(members)) => {
        let count = members.len();
        let tail = members.iter().map(format_bulk_string).collect::<String>();
        write!(
          formatter,
          "*{}\r\n$4\r\nSREM\r\n{}{}",
          count + 2,
          format_bulk_string(key),
          tail
        )
      }
      SetCommand::Pop(key, 1) => write!(formatter, "*2\r\n$4\r\nSPOP\r\n{}", format_bulk_string(key)),
      SetCommand::Pop(key, amt) => write!(
        formatter,
        "*2\r\n$4\r\nSPOP\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(amt)
      ),

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
    let cmd = SetCommand::Add::<_, &str>("seasons", Arity::One("one"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$4\r\nSADD\r\n$7\r\nseasons\r\n$3\r\none\r\n")
    );
  }

  #[test]
  fn test_sadd_multi() {
    let cmd = SetCommand::Add::<_, &str>("seasons", Arity::Many(vec!["one", "two"]));
    assert_eq!(
      format!("{}", cmd),
      String::from("*4\r\n$4\r\nSADD\r\n$7\r\nseasons\r\n$3\r\none\r\n$3\r\ntwo\r\n")
    );
  }

  #[test]
  fn test_smembers_multi() {
    let cmd = SetCommand::Members::<_, &str>("seasons");
    assert_eq!(
      format!("{}", cmd),
      String::from("*2\r\n$8\r\nSMEMBERS\r\n$7\r\nseasons\r\n")
    );
  }

  #[test]
  fn test_srem_single() {
    let cmd = SetCommand::Rem("seasons", Arity::One("one"));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$4\r\nSREM\r\n$7\r\nseasons\r\n$3\r\none\r\n")
    );
  }

  #[test]
  fn test_srem_multi() {
    let cmd = SetCommand::Rem("seasons", Arity::Many(vec!["one", "two"]));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*4\r\n$4\r\nSREM\r\n$7\r\nseasons\r\n$3\r\none\r\n$3\r\ntwo\r\n")
    );
  }

  #[test]
  fn test_scard_multi() {
    let cmd = SetCommand::Card::<_, &str>("seasons");
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*2\r\n$5\r\nSCARD\r\n$7\r\nseasons\r\n")
    );
  }

  #[test]
  fn test_sdiff_single() {
    let cmd = SetCommand::Diff::<_, &str>(Arity::One("one"));
    let mut buffer = Vec::new();
    write!(buffer, "{}", cmd).expect("was able to write");
    assert_eq!(
      String::from_utf8(buffer).unwrap(),
      String::from("*2\r\n$5\r\nSDIFF\r\n$3\r\none\r\n")
    );
  }

  #[test]
  fn test_sinter_single() {
    let cmd = SetCommand::Inter::<_, &str>(Arity::One("some"));
    assert_eq!(format!("{}", cmd), String::from("*2\r\n$6\r\nSINTER\r\n$4\r\nsome\r\n"));
  }

  #[test]
  fn test_sinter_multi() {
    let cmd = SetCommand::Inter::<_, &str>(Arity::Many(vec!["one", "two"]));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$6\r\nSINTER\r\n$3\r\none\r\n$3\r\ntwo\r\n")
    );
  }

  #[test]
  fn test_sdiff_multi() {
    let cmd = SetCommand::Diff::<_, &str>(Arity::Many(vec!["one", "two"]));
    assert_eq!(
      format!("{}", cmd),
      String::from("*3\r\n$5\r\nSDIFF\r\n$3\r\none\r\n$3\r\ntwo\r\n")
    );
  }
}
