# protots

Basic protobuf parser that is built to produce typescript type definitions
according to the message types defined in protobuf files. Preferably we want to
produce type schemas using [zod][zod].

> CAUTION: the current state is still to be considered alpha. Use at your own
> risk.


## Development


### Running

You can use the usual rust toolchain using `cargo`:

    $ cargo run -- ./some/file.proto


### Building

A release build ran be built via:

    $ cargo build --release


## Implementation notes

As of now, the implementation is loosely based on the official [protobuf language
spec](https://protobuf.dev/reference/protobuf/proto3-spec/) and the [text format
spec](https://protobuf.dev/reference/protobuf/textformat-spec). However that
spec is not accurate based on my experiments. The current `protoc` compiler(s)
allow way more than is described in the specs above.

Therefore I am basically just running tests on a bunch of protobuf files that I
found both on my machine and in the wild.

Bottom line, don't be surprised in case you have a proto file that is accepted
by the `protoc` but will not parse completely by `protots`.


## TODO

- thoroughly check protobuf specs and make sure that we support *at least*
  everything that is mentioned in there
- properly implement and extract field options (see
  [doc](https://protobuf.dev/programming-guides/proto3/#options))
- improve error handling/output
- add/extend unit tests


[zod]: https://github.com/colinhacks/zod
