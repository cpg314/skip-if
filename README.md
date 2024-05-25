# skip-if

This Rust [attribute macro](https://doc.rust-lang.org/reference/procedural-macros.html#attribute-macros) skips running a function that produces a file or a folder (`output`) depending on a `strategy`.

```rust
#[skip_if(output="avatars.join(name)", strategy="...")]
fn render_avatar(name: &str, avatars: &Path) -> anyhow::Result<()> {
  // This will be skipped depending on the strategy.
  // e.g. do not run if the output already exists or if the function previously
  // failed on these arguments.
  Ok(())
}
```

The strategy (specified in the `strategy` attribute) has access to:

- The arguments hash (excluding the ones provided in the `args_skip` attributes);
- The source code hash;
- The output path (as provided in the `output` attribute).
- A callback with a reference to the result of the method call.

For convenience, the function on which the `skip_if` attribute is applied can access the value of the `output` expression through the `skip_if_output` variable.

## Sample strategies

The simplest strategy, `skip_if::FileExists`, is to skip when the `output` path exists.

```rust
#[skip_if(output = "output", strategy = "skip_if::FileExists")]
```

A more advanced strategy, `skip_if::Markers`, offers the following options:

- Use a marker success file (`{output}.success`) with source and arguments hashes to never skip when these change, even if the output file already exists.
- Use a marker failure file (`{output}.failure`) to skip after a failure.
  - Optionally, take into account source and argument hashes to retry nevertheless when these change.
  - Optionally, mark some errors as retriable.
- Directory mode, where output is assumed to be a directory. The `success` and `failure` markers are stored inside.

```rust
#[skip_if(
  output = "output",
  strategy = "skip_if::Markers::default()",
  // or:
  // strategy = "skip_if::Markers::default()::folder()",
)]
```

## TODO

- `async` support.
- Allow selectively disabling code and argument hashing (they are now tied together).
