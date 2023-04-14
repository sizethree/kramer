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
  S: std::fmt::Display,
{
  /// The name of the ACL entry.
  pub name: S,

  /// An optional password that will be added to the acl command.
  pub password: Option<S>,

  /// The set of commands the ACL entry should have the ability to execute.
  pub commands: Option<Vec<S>>,

  /// The set of keys the ACL entry should have access to.
  pub keys: Option<S>,
}

/// Redis acl commands.
#[cfg(feature = "acl")]
#[derive(Debug)]
pub enum AclCommand<S>
where
  S: std::fmt::Display,
{
  /// Requests a list of all acl entries.
  List,

  /// Wraps the `SetUser` struct for a type implementing display.
  SetUser(SetUser<S>),

  /// Wraps the `DelUser` struct for a type implementing display.
  DelUser(Arity<S>),
}

#[cfg(feature = "acl")]
impl<S> std::fmt::Display for AclCommand<S>
where
  S: std::fmt::Display,
{
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      AclCommand::List => write!(formatter, "*2\r\n$3\r\nACL\r\n$4\r\nLIST\r\n"),
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
        (Some(password), Some(command_pattern), Some(key_pattern)) => {
          let count = 6 + command_pattern.len();
          write!(
            formatter,
            "*{count}\r\n$3\r\nACL\r\n{}{}{}{}{}{}",
            format_bulk_string("SETUSER"),
            format_bulk_string(&inner.name),
            format_bulk_string("on"),
            format_bulk_string(format!(">{password}")),
            format_bulk_string(format!("~{key_pattern}")),
            command_pattern.iter().fold(String::new(), |acc, command| acc
              + format_bulk_string(format!("+{command}")).as_str())
          )
        }
        // TODO: implement other combinations of this command.
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
  fn format_list() {
    let command: AclCommand<&str> = AclCommand::List;
    assert_eq!(format!("{command}"), "*2\r\n$3\r\nACL\r\n$4\r\nLIST\r\n");
  }

  #[test]
  fn format_full_setuser() {
    let command = AclCommand::SetUser(SetUser {
      name: "library-member",
      password: Some("many-books"),
      commands: Some(vec!["hgetall"]),
      keys: Some("books"),
    });

    assert_eq!(format!("{}", command), "*7\r\n$3\r\nACL\r\n$7\r\nSETUSER\r\n$14\r\nlibrary-member\r\n$2\r\non\r\n$11\r\n>many-books\r\n$6\r\n~books\r\n$8\r\n+hgetall\r\n");
    assert_eq!(
      humanize_command::<&str, &str>(&crate::Command::Acl(command)),
      "ACL SETUSER library-member on >many-books ~books +hgetall"
    );
  }

  #[test]
  fn format_full_setuser_multi_command() {
    let command = AclCommand::SetUser(SetUser {
      name: "library-member",
      password: Some("many-books"),
      commands: Some(vec!["hgetall", "blpop"]),
      keys: Some("books"),
    });

    assert_eq!(format!("{}", command), "*8\r\n$3\r\nACL\r\n$7\r\nSETUSER\r\n$14\r\nlibrary-member\r\n$2\r\non\r\n$11\r\n>many-books\r\n$6\r\n~books\r\n$8\r\n+hgetall\r\n$6\r\n+blpop\r\n");
    assert_eq!(
      humanize_command::<&str, &str>(&crate::Command::Acl(command)),
      "ACL SETUSER library-member on >many-books ~books +hgetall +blpop"
    );
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
