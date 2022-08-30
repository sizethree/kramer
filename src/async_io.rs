#![cfg(feature = "kramer-async")]

extern crate async_std;

use crate::response::{readline, Response, ResponseLine, ResponseValue};

use async_std::net::TcpStream;
use async_std::prelude::*;

use std::io::{Error, ErrorKind};

pub async fn read<C>(connection: C) -> Result<Response, Error>
where
  C: async_std::io::Read + std::marker::Unpin,
{
  let mut lines = async_std::io::BufReader::new(connection).lines();

  match lines
    .next()
    .await
    .ok_or_else(|| Error::new(ErrorKind::InvalidData, "kramer: no line available to parse."))
    .and_then(|res| res.and_then(readline))
  {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while let Ok(kind) = lines
        .next()
        .await
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "kramer: missing line during array parsing."))
        .and_then(|res| res.and_then(readline))
      {
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

      let out = lines.next().await.ok_or_else(|| {
        Error::new(
          ErrorKind::InvalidData,
          "kramer: expected line from bulk string but received nothing",
        )
      })??;

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
  C: async_std::io::Write + std::marker::Unpin + async_std::io::Read,
{
  connection.write_all(format!("{}", message).as_bytes()).await?;
  read(connection).await
}

pub async fn send<S>(addr: &str, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
{
  let mut stream = TcpStream::connect(addr).await?;
  execute(&mut stream, message).await
}
