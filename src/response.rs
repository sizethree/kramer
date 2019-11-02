use std::io::{Error, ErrorKind};

#[derive(Debug)]
pub enum ResponseLine {
  Array(usize),
  SimpleString(String),
  Error(String),
  Integer(i64),
  BulkString(usize),
  Null,
}

#[derive(Debug, PartialEq)]
pub enum ResponseValue {
  Empty,
  String(String),
  Integer(i64),
}

#[derive(Debug, PartialEq)]
pub enum Response {
  Array(Vec<ResponseValue>),
  Item(ResponseValue),
  Error,
}

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
      .map(|v| Some(v)),
  }
}

pub fn readline(result: Option<Result<String, Error>>) -> Result<ResponseLine, Error> {
  let line = result.ok_or_else(|| Error::new(ErrorKind::Other, "no line to work with"))??;

  match line.bytes().next() {
    Some(b'*') => match read_line_size(line)? {
      None => Ok(ResponseLine::Null),
      Some(size) => Ok(ResponseLine::Array(size)),
    },
    Some(b'$') => match read_line_size(line)? {
      Some(size) => Ok(ResponseLine::BulkString(size)),
      None => Ok(ResponseLine::Null),
    },
    Some(b'-') => Ok(ResponseLine::Error(line)),
    Some(b'+') => Ok(ResponseLine::SimpleString(String::from(line.split_at(1).1))),
    Some(b':') => {
      let (_, rest) = line.split_at(1);
      rest
        .parse::<i64>()
        .map_err(|e| Error::new(ErrorKind::Other, format!("{:?}", e)))
        .and_then(|v| Ok(ResponseLine::Integer(v)))
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
