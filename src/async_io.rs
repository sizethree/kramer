#![cfg(feature = "kramer-async")]
#![warn(clippy::print_stdout)]

extern crate async_std;

use crate::response::{readline, Response, ResponseLine, ResponseValue};

use async_std::net::TcpStream;
use async_std::prelude::*;

use std::io::{Error, ErrorKind};

/// Attempts to read RESP standard messages (newline delimeters), parsing into our `ResponseValue`
/// enum.
pub async fn read<C>(connection: C) -> Result<Response, Error>
where
  C: async_std::io::Read + std::marker::Unpin,
{
  let mut reader = async_std::io::BufReader::new(connection);
  let mut buffer = String::new();

  match reader.read_line(&mut buffer).await.and_then(|_res| readline(buffer)) {
    Ok(ResponseLine::Array(size)) => {
      let mut store = Vec::with_capacity(size);

      if size == 0 {
        return Ok(Response::Array(vec![]));
      }

      while store.len() < size {
        let mut line_buffer = String::new();

        let kind = reader
          .read_line(&mut line_buffer)
          .await
          .and_then(|_res| readline(line_buffer))?;

        match kind {
          ResponseLine::BulkString(size) => {
            let mut real_value = String::with_capacity(size);
            reader.read_line(&mut real_value).await?;
            store.push(ResponseValue::String(real_value.trim_end().to_string()));
          }
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

      let mut real_value = String::with_capacity(size);
      reader.read_line(&mut real_value).await?;

      Ok(Response::Item(ResponseValue::String(real_value.trim_end().to_string())))
    }
    Ok(ResponseLine::Null) => Ok(Response::Item(ResponseValue::Empty)),
    Ok(ResponseLine::SimpleString(simple)) => Ok(Response::Item(ResponseValue::String(simple.trim_end().to_string()))),
    Ok(ResponseLine::Integer(value)) => Ok(Response::Item(ResponseValue::Integer(value))),
    Ok(ResponseLine::Error(e)) => Err(Error::new(ErrorKind::Other, e)),
    Err(e) => Err(e),
  }
}

/// An async implementation of a complete message exchange. The provided message will be written to
/// our connection, and a response will be read.
pub async fn execute<C, S>(mut connection: C, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
  C: async_std::io::Write + std::marker::Unpin + async_std::io::Read,
{
  connection.write_all(format!("{}", message).as_bytes()).await?;
  read(connection).await
}

/// An async implementation of opening a tcp connection, and sending a single message.
pub async fn send<S>(addr: &str, message: S) -> Result<Response, Error>
where
  S: std::fmt::Display,
{
  let mut stream = TcpStream::connect(addr).await?;
  execute(&mut stream, message).await
}
