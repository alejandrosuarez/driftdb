# DriftDB

[![GitHub Repo stars](https://img.shields.io/github/stars/drifting-in-space/driftdb?style=social)](https://github.com/drifting-in-space/driftdb)
[![crates.io](https://img.shields.io/crates/v/driftdb.svg)](https://crates.io/crates/driftdb)
[![docs.rs](https://img.shields.io/badge/rust-docs-brightgreen)](https://docs.rs/driftdb/)
[![docs.rs](https://img.shields.io/badge/client-docs-brightgreen)](https://driftdb.com/)
[![wokflow state](https://github.com/drifting-in-space/driftdb/workflows/build/badge.svg)](https://github.com/drifting-in-space/driftdb/actions/workflows/test.yml)

DriftDB is a real-time data backend for browser-based applications.

For more information, see [driftdb.com](https://driftdb.com).

## Structure of this repo

- `demos/`: next.js project containing several demos, available on the web at [demos.driftdb.com](https://demos.driftdb.com).
- `docs/`: main online documentation and website for driftdb, available on the web at [driftdb.com](https://driftdb.com).
- `driftdb/`: core Rust driftdb implementation.
- `driftdb-server/`: Rust crate of driftdb dev server.
- `driftdb-worker/`: driftdb implementation on Cloudflare Workers.
- `driftdb-ui/`: React-based debug UI for driftdb.
- `js-lib/`: JavaScript client library for driftdb.
