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
      if v.starts_with("$") || v.starts_with("*") {
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
