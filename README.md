# Cargo `pkg-config` tool
This tool extends Cargo with a pkg-config-like interface.

This allows you to extract metadata from Cargo crates like build artifacts
in a manner that is synonymous with the native `pkg-config`.

For example, with `bindgen`:
```
> cargo pkgconfig --libs bindgen
    Finished dev [unoptimized + debuginfo] target(s) in 0.05s
/LIBPATH:D:/Dev/Repositories/rust-bindgen/target/debug libbindgen.rlib Bcrypt.lib Userenv.lib
```

You can then use this output such as follows in a Makefile project:
```shell
# Declare the Rust crate to always be dirty and let Cargo handle rebuilds.
# It'd be nice to figure out a way to let Cargo tell make if the crate was rebuilt (to clean up stdout),
# but seemingly challenging to do so.
.PHONY: libsync.a
libsync.a:
	@RMCOMMAND@ libsync.a
	CRATELIBS=`@CARGO@ pkgconfig --libs sync -- --release --manifest-path $(srcdir)/sync/Cargo.toml`; \
	if [ $$? -ne 0 ]; then exit $$?; fi; \
	@MAKELIB@ $$CRATELIBS; \
	$(RANLIB) libsync.a

libcpu.a: $(OBJS) @OBJS64@ libsync.a
	@RMCOMMAND@ libcpu.a
	@MAKELIB@ $(OBJS) @OBJS64@ libsync.a
	$(RANLIB) libcpu.a
```

Note that this is not to be confused with the [`pkg-config`](https://crates.io/crates/pkg-config) crate,
which is intended to expose a programmatic interface to the native `pkg-config` for `build.rs` scripts.
