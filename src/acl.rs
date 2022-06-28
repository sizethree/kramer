//! Notice: This feature is still not fully implemented and gated behind the `acl` feature flag for
//! now. The current implementation is designed to _only_ satisfy the single use case so far of
//! creating an acl entry for a user with a password, command and keys list, e.g:
//!
//! ```redis
//! ACL SETUSER on my-user >my-password ~keys +commands
//! ```
//!
//! This means that the `SetUser` command (with it's respective struct) is only partially
//! implemented until it is clear what exactly the other variations of it would mean.
//!
//! [`SETUSER` docs](https://redis.io/commands/acl-setuser/)

use super::modifiers::{format_bulk_string, Arity};

/// Notice: Currently `Display` is only implemented if all fields are present/`Some`.
#[cfg(feature = "acl")]
#[derive(Debug)]
pub struct SetUser<S>
where
  S:,
{
  pub name: S,
  pub password: Option<S>,
  pub commands: Option<S>,
  pub keys: Option<S>,
}

#[cfg(feature = "acl")]
#[derive(Debug)]
pub enum AclCommand<S>
where
  S: std::fmt::Display,
{
  SetUser(SetUser<S>),
  DelUser(Arity<S>),
}

#[cfg(feature = "acl")]
impl<S> std::fmt::Display for AclCommand<S>
where
  S: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      AclCommand::DelUser(Arity::One(inner)) => {
        write!(
          formatter,
          "*3\r\n$3\r\nACL\r\n{}{}",
          format_bulk_string("DELUSER"),
          format_bulk_string(inner)
        )
      }
      AclCommand::DelUser(Arity::Many(inner)) => {
        let amount = 2 + inner.len();
        write!(
          formatter,
          "*{}\r\n$3\r\nACL\r\n{}{}",
          amount,
          format_bulk_string("DELUSER"),
          inner.iter().map(format_bulk_string).collect::<String>(),
        )
      }
      AclCommand::SetUser(inner) => match (&inner.password, &inner.commands, &inner.keys) {
        (Some(p), Some(c), Some(k)) => {
          write!(
            formatter,
            "*6\r\n$3\r\nACL\r\n{}{}{}{}{}{}",
            format_bulk_string("SETUSER"),
            format_bulk_string(&inner.name),
            format_bulk_string("on"),
            format_bulk_string(format!(">{}", p)),
            format_bulk_string(format!("+{}", c)),
            format_bulk_string(format!("~{}", k)),
          )
        }
        (_, _, _) => Ok(()),
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{AclCommand, SetUser};
  use crate::modifiers::{humanize_command, Arity};

  #[test]
  fn format_full_setuser() {
    let command = AclCommand::SetUser(SetUser {
      name: "library-member",
      password: Some("many-books"),
      commands: Some("hgetall"),
      keys: Some("books"),
    });

    assert_eq!(format!("{}", command), "*6\r\n$3\r\nACL\r\n$7\r\nSETUSER\r\n$14\r\nlibrary-member\r\n$2\r\non\r\n$11\r\n>many-books\r\n$8\r\n+hgetall\r\n$6\r\n~books\r\n");
  }

  #[test]
  fn format_deluser_one() {
    let command = AclCommand::DelUser(Arity::One("my-user"));

    assert_eq!(
      format!("{}", command),
      "*3\r\n$3\r\nACL\r\n$7\r\nDELUSER\r\n$7\r\nmy-user\r\n"
    );

    assert_eq!(
      humanize_command::<&str, &str>(&crate::Command::Acl(command)),
      "ACL DELUSER my-user"
    );
  }

  #[test]
  fn format_deluser_many() {
    let command = AclCommand::DelUser(Arity::Many(vec!["my-user", "other-user"]));

    assert_eq!(
      format!("{}", command),
      "*4\r\n$3\r\nACL\r\n$7\r\nDELUSER\r\n$7\r\nmy-user\r\n$10\r\nother-user\r\n"
    );
    assert_eq!(
      humanize_command::<&str, &str>(&crate::Command::Acl(command)),
      "ACL DELUSER my-user other-user"
    );
  }

  #[test]
  fn format_partial_setuser() {
    let command = AclCommand::SetUser(SetUser {
      name: "library-member",
      password: None,
      commands: None,
      keys: None,
    });

    assert_eq!(format!("{}", command), "")
  }
}
