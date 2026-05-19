# Fuzz Targets

Build the Hono bundles before fuzzing:

```bash
npm --prefix examples/hono-status install
npm --prefix examples/hono-status run build
npm --prefix examples/hono-suite install
npm --prefix examples/hono-suite run build
```

Run the Hono API fuzz targets:

```bash
cargo install cargo-fuzz
cargo fuzz run hono_status_api -- -max_total_time=60 -max_len=512
cargo fuzz run hono_suite_api -- -max_total_time=60 -max_len=512
```

On stable Rust without sanitizer support:

```bash
cargo fuzz run hono_status_api --sanitizer none -- -max_total_time=60 -max_len=512
cargo fuzz run hono_suite_api --sanitizer none -- -max_total_time=60 -max_len=512
```
