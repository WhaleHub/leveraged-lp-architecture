# Interfaces

Contract surfaces only — **no implementations**. Two kinds of file:

| | |
|---|---|
| `leverage_vault.rs`, `zapper.rs`, `lp_oracle_adapter.rs` | What this design *defines*. Traits + error enums + storage types. |
| `blend_pool.rs`, `amm_pool.rs` | What it *consumes*. The subset of Blend v2 and Aquarius actually used, so a reader can follow the call graph without cloning either. |

These do not compile as a crate and are not meant to: there is no `Cargo.toml`,
no state, no bodies. They are here to be read alongside the
[architecture README](../README.md), and to be copied as a starting point by
anyone building the same thing.

The doc comments carry the parts that cost real debugging time — the
re-entrancy constraint, the `transfer_from` vs `transfer` authorization split,
the error-code collision at `#4`, and why a zap floor near the borrow amount can
never be met. Read those before the signatures.
