# Cargo `pkg-config` tool
This tool extends Cargo with a pkg-config-like interface.

This allows you to extract metadata from Cargo crates like build artifacts
in a manner that is synonymous with the native `pkg-config`.

For example, with `bindgen`:
```
$ cargo pkg-config --libs bindgen
-LD:\Dev\Repositories\rust-bindgen\target\release -lbindgen
```
