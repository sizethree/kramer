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
