/// For lists, items can either be inserted on the left or right; this translates to
/// whether or not the generated command is `LPOP` or `RPOP` (for example).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
  /// Insert at the start.
  Left,

  /// Insert at the end.
  Right,
}

/// Redis provides the ability to conditionally apply an inseration based on the existence
/// of the a value that is equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Insertion {
  /// The presence of the value does not matter.
  Always,

  /// The presence of the value is required to insert.
  IfExists,

  /// The presence of the value is forbidden to insert.
  IfNotExists,
}

/// The arity type here is used to mean a single or non-single container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arity<S> {
  /// Wraps a `Vec`; many values.
  Many(Vec<S>),

  /// Indicates a single value.
  One(S),
}

/// This method will return a string that is formatted following the redis serialization protocol
/// standard to represent a bulk string.
pub fn format_bulk_string<S: std::fmt::Display>(input: S) -> String {
  let as_str = format!("{}", input);
  format!("${}\r\n{}\r\n", as_str.len(), as_str)
}

/// By default, all commands will be formatted via the `Display` trait into the string
/// representation that they would be sent over the wire as. This function should help users
/// visualize commands in the format that they would issue them into the `redis-cli` as.
pub fn humanize_command<S, V>(input: &super::Command<S, V>) -> String
where
  S: std::fmt::Display,
  V: std::fmt::Display,
{
  let as_str = format!("{}", input);
  as_str
    .split("\r\n")
    .filter_map(|v| {
      if v.starts_with('$') || v.starts_with('*') {
        None
      } else {
        Some(format!("{} ", v))
      }
    })
    .collect::<String>()
    .trim_end()
    .to_string()
}

#[cfg(test)]
mod tests {
  use super::humanize_command;

  #[test]
  fn test_humanize() {
    let command = crate::Command::Auth::<&str, &str>(crate::AuthCredentials::User(("testing", "testerton")));
    let humanized = humanize_command(&command);
    assert_eq!(humanized, "AUTH testing testerton");
  }
}
