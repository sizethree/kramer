## kramer

[![ci.img]][ci.url] [![docs.img]][docs.url] [![crates.img]][crates.url]

An implementation of the [redis protocol specification][redis] with an execution helper using the
[`TcpStream`][tcp-stream] provided by [async-std].

For a list of supported commands see [todo.md](/.todo.md).

| kramer |
| --- |
| ![kramer][kramer] |

## Contributing

See [CONTRIBUTING](/CONTRIBUTING.md).

[ci.img]: https://github.com/sizethree/kramer/workflows/gh.build/badge.svg?flat
[ci.url]: https://github.com/sizethree/kramer/actions?workflow=gh.build
[redis]: https://redis.io/topics/protocol
[async-std]: https://github.com/async-rs/async-std
[tcp-stream]: https://docs.rs/async-std/0.99.11/async_std/net/struct.TcpStream.html
[docs.img]: https://docs.rs/kramer/badge.svg
[docs.url]: https://docs.rs/kramer/latest
[crates.url]: https://crates.io/crates/kramer
[crates.img]: https://img.shields.io/crates/v/kramer
[kramer]: https://user-images.githubusercontent.com/1545348/68049259-d341d600-fcb8-11e9-9f25-e1bcf122cd59.gif
