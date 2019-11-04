use crate::modifiers::{format_bulk_string, Arity, Insertion, Side};

#[derive(Debug)]
pub enum ListCommand<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  Len(S),
  Push((Side, Insertion), S, Arity<V>),
  Pop(Side, S, Option<(Option<Arity<S>>, u64)>),
  Rem(S, V, u64),
  Index(S, i64),
  Set(S, u64, V),
  Insert(S, Side, V, V),
  Trim(S, i64, i64),
  Range(S, i64, i64),
}

impl<S, V> std::fmt::Display for ListCommand<S, V>
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      ListCommand::Trim(key, start, stop) => {
        let tail = format!(
          "{}{}{}",
          format_bulk_string(key),
          format_bulk_string(start),
          format_bulk_string(stop)
        );
        write!(formatter, "*4\r\n$5\r\nLTRIM\r\n{}", tail)
      }
      ListCommand::Set(key, index, element) => {
        let tail = format!(
          "{}{}{}",
          format_bulk_string(key),
          format_bulk_string(index),
          format_bulk_string(element)
        );
        write!(formatter, "*4\r\n$4\r\nLSET\r\n{}", tail)
      }
      ListCommand::Insert(key, side, pivot, element) => {
        let side = match side {
          Side::Left => format_bulk_string("BEFORE"),
          Side::Right => format_bulk_string("AFTER"),
        };
        let tail = format!("{}{}", format_bulk_string(pivot), format_bulk_string(element));

        write!(
          formatter,
          "*5\r\n$7\r\nLINSERT\r\n{}{}{}",
          format_bulk_string(key),
          side,
          tail,
        )
      }
      ListCommand::Index(key, amt) => {
        let tail = format!("{}{}", format_bulk_string(key), format_bulk_string(amt));
        write!(formatter, "*3\r\n$6\r\nLINDEX\r\n{}", tail)
      }
      ListCommand::Rem(key, value, count) => {
        let end = format!(
          "{}{}{}",
          format_bulk_string(key),
          format_bulk_string(count),
          format_bulk_string(value),
        );

        write!(formatter, "*4\r\n$4\r\nLREM\r\n{}", end)
      }
      ListCommand::Range(key, from, to) => {
        let end = format!("{}{}", format_bulk_string(from), format_bulk_string(to));
        write!(formatter, "*4\r\n$6\r\nLRANGE\r\n{}{}", format_bulk_string(key), end)
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
