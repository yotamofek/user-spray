# user-spray
Group "use" imports, while the feature is still unstable on rustfmt. 🔫

## Introduction

This is a simple tool that tries to mimick the unstable [`group_imports`](https://rust-lang.github.io/rustfmt/?version=v1.6.0&search=#group_imports) feature of `rustfmt`, for usage in projects using the stable rust toolchain.

It is very much a WIP and only works for very basic cases, I'm dogfooding it on my own projects and will fix/implement more stuff as I go.

## Usage

The binary is meant to be used instead of `rustfmt` (NOT `cargo fmt`), and will run `rustfmt` on its input after it is done. The main purpose of this is to allow easy integration with `rust-analyzer`.

Install `user-spray`:
```sh
cargo install --git https://github.com/yotamofek/user-spray
```

And then add the following configuration to your VSCode's `settings.json`:
```json
{
    "rust-analyzer.rustfmt.overrideCommand": [
        "user-spray",
        "--",
        // r-a passes this flag to rustfmt automatically, but since we override the command we need to add it manually
        "--edition=2021"
    ]
}
```

To skip running `rustfmt` on the output, use the `--skip-rustfmt` flag.

## Example

Before:
```rust
use std::io::Write;
use mycrate::Foo;
use std::collections::{HashMap, HashSet};
use self::mymod::Bar;
pub use std::io::{self, Read};
pub(super) use self::mymod::HelloWorld;
```

After (with `--skip-rustfmt`):
```rust
use std::{io::{Write}, collections::{HashMap, HashSet}};
pub use std::{io::{self, Read}};

use mycrate::{Foo};

use self::{mymod::{Bar}};
pub(super) use self::{mymod::{HelloWorld}};
```

After `rustfmt`:
```rust
pub use std::io::{self, Read};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
};

use mycrate::Foo;

use self::mymod::Bar;
pub(super) use self::mymod::HelloWorld;
```

## Todo

- [X] Globs
- [X] Renames
- [X] Restricted visibility (e.g. `pub(crate) use self::mymod::Bar`)
- [ ] `use` items inside other items (such as `mod`, `fn`s, etc.)
- [ ] Handle doc comments on `use` items
- [ ] Tests!
