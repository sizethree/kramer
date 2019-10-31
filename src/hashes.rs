use crate::modifiers::{format_bulk_string, Arity, Insertion};

#[derive(Debug)]
pub enum HashCommand<S>
where
  S: std::fmt::Display,
{
  Del(S, Arity<S>),
  Set(S, Arity<(S, S)>, Insertion),
  Get(S, Option<Arity<S>>),
  StrLen(S, S),
  Len(S),
  Incr(S, S, i64),
  Keys(S),
  Vals(S),
  Exists(S, S),
}

impl<S: std::fmt::Display> std::fmt::Display for HashCommand<S> {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      HashCommand::StrLen(key, field) => {
        let tail = format!("{}{}", format_bulk_string(key), format_bulk_string(field));
        write!(formatter, "*3\r\n$7\r\nHSTRLEN\r\n{}", tail)
      }
      HashCommand::Incr(key, field, amt) => {
        let tail = format!(
          "{}{}{}",
          format_bulk_string(key),
          format_bulk_string(field),
          format_bulk_string(amt)
        );
        write!(formatter, "*4\r\n$7\r\nHINCRBY\r\n{}", tail)
      }
      HashCommand::Vals(key) => write!(formatter, "*2\r\n$5\r\nHVALS\r\n{}", format_bulk_string(key)),
      HashCommand::Keys(key) => write!(formatter, "*2\r\n$5\r\nHKEYS\r\n{}", format_bulk_string(key)),
      HashCommand::Len(key) => write!(formatter, "*2\r\n$4\r\nHLEN\r\n{}", format_bulk_string(key)),
      HashCommand::Get(key, None) => write!(formatter, "*2\r\n$7\r\nHGETALL\r\n{}", format_bulk_string(key)),
      HashCommand::Get(key, Some(Arity::One(field))) => write!(
        formatter,
        "*3\r\n$4\r\nHGET\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(field)
      ),
      HashCommand::Get(key, Some(Arity::Many(fields))) => {
        let len = fields.len();

        // Awkward; Get("foo", Some(Arity::Many(vec![]))) == Get("foo", None)
        if len == 0 {
          let formatted = format!("{}", key);
          return write!(formatter, "{}", HashCommand::Get(formatted, None));
        }

        let tail = fields.iter().map(format_bulk_string).collect::<String>();

        write!(
          formatter,
          "*{}\r\n$5\r\nHMGET\r\n{}{}",
          2 + len,
          format_bulk_string(key),
          tail
        )
      }
      HashCommand::Exists(key, field) => write!(
        formatter,
        "*3\r\n$7\r\nHEXISTS\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(field)
      ),
      HashCommand::Set(key, Arity::One((field, value)), Insertion::IfNotExists) => write!(
        formatter,
        "*4\r\n$6\r\nHSETNX\r\n{}{}{}",
        format_bulk_string(key),
        format_bulk_string(field),
        format_bulk_string(value)
      ),
      HashCommand::Set(key, Arity::Many(mappings), Insertion::IfNotExists) => {
        let count = mappings.len();
        let tail = mappings
          .iter()
          .map(|(k, v)| format!("{}{}", format_bulk_string(k), format_bulk_string(v)))
          .collect::<String>();

        write!(
          formatter,
          "*{}\r\n$6\r\nHSETNX\r\n{}{}",
          2 + (count * 2),
          format_bulk_string(key),
          tail
        )
      }
      HashCommand::Set(key, Arity::One((field, value)), _) => write!(
        formatter,
        "*4\r\n$4\r\nHSET\r\n{}{}{}",
        format_bulk_string(key),
        format_bulk_string(field),
        format_bulk_string(value)
      ),
      HashCommand::Set(key, Arity::Many(mappings), _) => {
        let count = mappings.len();
        let tail = mappings
          .iter()
          .map(|(k, v)| format!("{}{}", format_bulk_string(k), format_bulk_string(v)))
          .collect::<String>();

        write!(
          formatter,
          "*{}\r\n$4\r\nHSET\r\n{}{}",
          2 + (count * 2),
          format_bulk_string(key),
          tail
        )
      }
      HashCommand::Del(key, Arity::One(field)) => write!(
        formatter,
        "*3\r\n$4\r\nHDEL\r\n{}{}",
        format_bulk_string(key),
        format_bulk_string(field)
      ),
      HashCommand::Del(key, Arity::Many(fields)) => {
        let count = fields.len();
        let bits = fields.iter().map(format_bulk_string).collect::<String>();
        write!(
          formatter,
          "*{}\r\n$4\r\nHDEL\r\n{}{}",
          count + 2,
          format_bulk_string(key),
          bits
        )
      }
    }
  }
}
