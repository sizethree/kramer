#![cfg(feature = "kramer-io")]

extern crate async_std;

use async_std::net::TcpStream;
use async_std::prelude::*;

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

fn readline(result: Option<Result<String, Error>>) -> Result<ResponseLine, Error> {
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

pub async fn read<C>(connection: C) -> Result<Response, Error>
where
  C: async_std::io::Read + std::marker::Unpin,
{
  let mut lines = async_std::io::BufReader::new(connection).lines();

  match readline(lines.next().await) {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while let Ok(kind) = readline(lines.next().await) {
        match kind {
          ResponseLine::BulkString(size) => match lines.next().await {
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
        .await
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

pub async fn execute<C, S>(mut connection: C, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
  C: async_std::io::Write + async_std::io::Read + std::marker::Unpin,
{
  write!(connection, "{}", message).await?;
  read(connection).await
}

pub async fn send<S>(addr: &str, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
{
  let mut stream = TcpStream::connect(addr).await?;
  execute(&mut stream, message).await
}
