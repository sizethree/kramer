use crate::response::{readline, Response, ResponseLine, ResponseValue};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

/// After sending a command, the read here is used to parse the response from our connection
/// into the response enum.
pub fn read<C>(read: C) -> Result<Response, Error>
where
  C: std::io::Read + std::marker::Unpin,
{
  let mut lines = std::io::BufReader::new(read).lines();

  match lines
    .next()
    .ok_or_else(|| Error::new(ErrorKind::NotFound, "kramer: No lines available from reader."))
    .and_then(|opt| opt.and_then(readline))
  {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while let Ok(kind) = lines
        .next()
        .ok_or_else(|| {
          Error::new(
            ErrorKind::InvalidData,
            "kramer: No lines avaible during array response parsing.",
          )
        })
        .and_then(|opt| opt.and_then(readline))
      {
        match kind {
          ResponseLine::BulkString(size) => match lines.next() {
            Some(Ok(bulky)) if bulky.len() == size => {
              store.push(ResponseValue::String(bulky));
            }
            _ => break,
          },
          _ => break,
        }

        if store.len() >= size {
          return Ok(Response::Array(store));
        }
      }

      if size != store.len() {
        let message = format!("expected {} elements in response and received {}", size, store.len());
        return Err(Error::new(ErrorKind::InvalidData, message));
      }

      Ok(Response::Array(store))
    }
    Ok(ResponseLine::BulkString(size)) => {
      if size < 1 {
        return Ok(Response::Item(ResponseValue::Empty));
      }

      let out = lines
        .next()
        .ok_or_else(|| Error::new(ErrorKind::Other, "no line to work with"))??;

      Ok(Response::Item(ResponseValue::String(out)))
    }
    Ok(ResponseLine::Null) => Ok(Response::Item(ResponseValue::Empty)),
    Ok(ResponseLine::SimpleString(simple)) => Ok(Response::Item(ResponseValue::String(simple))),
    Ok(ResponseLine::Integer(value)) => Ok(Response::Item(ResponseValue::Integer(value))),
    Ok(ResponseLine::Error(e)) => Err(Error::new(ErrorKind::Other, e)),
    Err(e) => Err(e),
  }
}

/// Writes a command to the connection and will attempt to read a response.
pub fn execute<C, S>(mut connection: C, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
  C: std::io::Write + std::io::Read + std::marker::Unpin,
{
  write!(connection, "{}", message)?;
  read(connection)
}

/// This method will attempt to establish a _new_ connection and execute the command.
pub fn send<S>(addr: &str, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
{
  let mut stream = std::net::TcpStream::connect(addr)?;
  execute(&mut stream, message)
}
