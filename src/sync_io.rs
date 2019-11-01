use crate::response::{readline, Response, ResponseLine, ResponseValue};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};

pub fn read<C>(read: C) -> Result<Response, Error>
where
  C: std::io::Read + std::marker::Unpin,
{
  let mut lines = std::io::BufReader::new(read).lines();

  match readline(lines.next()) {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while let Ok(kind) = readline(lines.next()) {
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

pub fn execute<C, S>(mut connection: C, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
  C: std::io::Write + std::io::Read + std::marker::Unpin,
{
  write!(connection, "{}", message)?;
  read(connection)
}

pub fn send<S>(addr: &str, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
{
  let mut stream = std::net::TcpStream::connect(addr)?;
  execute(&mut stream, message)
}
