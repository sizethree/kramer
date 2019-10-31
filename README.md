## kramer

[![ci.img]][ci.url] [![docs.img]][docs.url]

An implementation of the [redis protocol specification][redis] with an execution helper using the
[`TcpStream`][tcp-stream] provided by [async-std].


For a list of supported commands see [todo.md](/.todo.md).

## Contributing

See [CONTRIBUTING](/CONTRIBUTING.md).

[ci.img]: https://github.com/sizethree/kramer/workflows/gh.build/badge.svg?flat
[ci.url]: https://github.com/sizethree/kramer/actions?workflow=gh.build
[redis]: https://redis.io/topics/protocol
[async-std]: https://github.com/async-rs/async-std
[tcp-stream]: https://docs.rs/async-std/0.99.11/async_std/net/struct.TcpStream.html
[docs.img]: https://docs.rs/kramer/badge.svg
[docs.url]: https://docs.rs/kramer
