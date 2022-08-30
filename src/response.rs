use std::io::{Error, ErrorKind};

/// A response line is the type that is parsed from a single `\r\n` delimited string returned from
/// the redis server.
#[derive(Debug)]
pub enum ResponseLine {
  /// An array response line indicates we have a string following.
  Array(usize),

  /// A simple string line is typically predicated by a bulk string line.
  SimpleString(String),

  /// The error line includes a message.
  Error(String),

  /// Integers - signed.
  Integer(i64),

  /// The bulk string response is usually part of an array.
  BulkString(usize),

  /// A null response line.
  Null,
}

/// A redis response value may either be empty, a bulk string, or an integer.
#[derive(Debug, PartialEq, Eq)]
pub enum ResponseValue {
  /// The empty response.
  Empty,

  /// Bulk string responses.
  String(String),

  /// Integer responses.
  Integer(i64),
}

/// Redis responses may either be an array of values, a single value, or an error.
#[derive(Debug, PartialEq, Eq)]
pub enum Response {
  /// A multi value response.
  Array(Vec<ResponseValue>),

  /// A single value.
  Item(ResponseValue),

  /// The error message returned from redis.
  Error,
}

/// Most redis responses will be a bulk string, or an integer. In either case, we want to parse
/// this as a usize and return that value. We're also translating from an integer `-1` value into a
/// `None` to represent an empty value.
fn read_line_size(line: String) -> Result<Option<usize>, Error> {
  match line.split_at(1).1 {
    "-1" => Ok(None),
    value => value
      .parse::<usize>()
      .map_err(|e| {
        Error::new(
          ErrorKind::Other,
          format!("invalid array length value '{}': {}", line.as_str(), e),
        )
      })
      .map(Some),
  }
}

/// Given a string, this method will attempt to parse it into our `ResponseLine` enum.
pub fn readline(result: String) -> Result<ResponseLine, Error> {
  match result.bytes().next() {
    Some(b'*') => match read_line_size(result)? {
      None => Ok(ResponseLine::Null),
      Some(size) => Ok(ResponseLine::Array(size)),
    },
    Some(b'$') => match read_line_size(result)? {
      Some(size) => Ok(ResponseLine::BulkString(size)),
      None => Ok(ResponseLine::Null),
    },
    Some(b'-') => Ok(ResponseLine::Error(result)),
    Some(b'+') => Ok(ResponseLine::SimpleString(String::from(result.split_at(1).1))),
    Some(b':') => {
      let (_, rest) = result.split_at(1);
      rest
        .parse::<i64>()
        .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))
        .map(ResponseLine::Integer)
    }
    Some(unknown) => Err(Error::new(
      ErrorKind::Other,
      format!("invalid message byte leader: {}", unknown),
    )),
    None => Err(Error::new(
      ErrorKind::Other,
      "empty line in response, unable to determine type",
    )),
  }
}
