# protots

[![protots](https://github.com/kongo2002/protots/actions/workflows/build.yml/badge.svg)][actions]

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


## Example


### Input

```proto
syntax = "proto3";

message Test {
  optional int32 version = 1;
  repeated string names = 2;

  oneof option {
    string foo = 3;
    int64 bar = 4;
  }
}
```


### Output

```typescript
import { z } from "zod";

export const TestSchema = z.object({
  version: z.optional(z.number()),
  names: z.array(z.string()),
  option: z.union([z.object({ foo: z.string() }), z.object({ bar: z.coerce.bigint() })]),
});

export type Test = z.infer<typeof TestSchema>;
```


## TODO

- process all protobuf files in a directory tree at once
- thoroughly check protobuf specs and make sure that we support *at least*
  everything that is mentioned in there
- properly implement and extract field options (see
  [doc](https://protobuf.dev/programming-guides/proto3/#options))
- improve error handling/output
- extend unit tests


[actions]: https://github.com/kongo2002/protots/actions/
[zod]: https://github.com/colinhacks/zod
