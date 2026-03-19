# Plogtion

A collection of simple functions that takes a form, uploads the images to a bucket, and commits a new post to my Plog.

They are each deployed as serverless functions at Scaleway.

## Development

Requires [mise](https://mise.jdx.dev/) for toolchain management.

```
mise install
cargo test --workspace
```

Run locally with:

```
cd local && cargo run
```
