use crate::modifiers::{format_bulk_string, Arity, Insertion};

#[derive(Debug)]
pub enum StringCommand<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  Set(Arity<(S, V)>, Option<std::time::Duration>, Insertion),
  Get(Arity<S>),
  Len(S),
  Decr(S, usize),
  Incr(S, i64),
  Append(S, V),
}

impl<S, V> std::fmt::Display for StringCommand<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      StringCommand::Len(key) => write!(formatter, "*2\r\n$6\r\nSTRLEN\r\n{}", format_bulk_string(key)),
      StringCommand::Incr(key, 1) => write!(formatter, "*2\r\n$4\r\nINCR\r\n{}", format_bulk_string(key)),
      StringCommand::Incr(key, amt) => write!(
        formatter,
        "*3\r\n$6\r\nINCRBY\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(amt)
      ),
      StringCommand::Decr(key, 1) => write!(formatter, "*2\r\n$4\r\nDECR\r\n{}", format_bulk_string(key)),
      StringCommand::Decr(key, amt) => write!(
        formatter,
        "*3\r\n$6\r\nDECRBY\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(amt)
      ),
      StringCommand::Get(Arity::One(key)) => write!(formatter, "*2\r\n$3\r\nGET\r\n{}", format_bulk_string(key)),
      StringCommand::Get(Arity::Many(keys)) => {
        let count = keys.len();
        let tail = keys.iter().map(format_bulk_string).collect::<String>();
        write!(formatter, "*{}\r\n$4\r\nMGET\r\n{}", count + 1, tail)
      }
      StringCommand::Append(key, value) => write!(
        formatter,
        "*3\r\n$6\r\nAPPEND\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(value)
      ),
      StringCommand::Set(Arity::One((key, value)), timeout, insertion) => {
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
      // Timeouts are not supported with a many set.
      StringCommand::Set(Arity::Many(assignments), _, insertion) => {
        let count = (assignments.len() * 2) + 1;
        let cmd = match insertion {
          Insertion::IfNotExists => "MSETNX",
          _ => "MSET",
        };
        let tail = assignments
          .iter()
          .map(|(k, v)| format!("{}{}", format_bulk_string(k), format_bulk_string(v)))
          .collect::<String>();
        write!(formatter, "*{}\r\n{}{}", count, format_bulk_string(cmd), tail)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::StringCommand;

  #[test]
  fn tes_strlen_present() {
    let cmd = StringCommand::Len::<_, &str>("seinfeld");
    assert_eq!(
      format!("{}", cmd),
      String::from("*2\r\n$6\r\nSTRLEN\r\n$8\r\nseinfeld\r\n")
    );
  }
}
