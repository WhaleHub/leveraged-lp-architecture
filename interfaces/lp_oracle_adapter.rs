//! Fair-LP oracle adapter — interface only, no implementation.
//!
//! THE RULE
//! --------
//! The LP price NEVER reads the AMM's own spot price or a TWAP of it. A lending
//! market that prices collateral from a number an attacker can move with a flash
//! loan can be drained. Price from the pool INVARIANT plus independent feeds:
//!
//!     LP_price = 2 * sqrt(K * P_a * P_b) / L        where K = x * y
//!
//! Pool state enters only through `K`. Reserves are cheap to shove around; `K`
//! moves only if the attacker deposits real assets and leaves them behind. This
//! is the fair-value formula used by Alpha Homora, Curve and Chainlink
//! reference oracles.
//!
//! PREFER THE POOL'S OWN QUOTE
//! ---------------------------
//! Where the AMM exposes an `estimate_swap`-style read, that is the contract's
//! own number: no amplification convention, no fee assumption, and immune to a
//! governance `ramp_a` changing the curve underneath a hardcoded constant. Use
//! the formula above as the FALLBACK path, not the primary one.

#![no_std]

use soroban_sdk::{contractclient, contracterror, contracttype, Address, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    /// No usable reference: every source failed or is out of band.
    NoReference = 2,
    /// A feed is older than `max_age_seconds`.
    StalePrice = 3,
    /// Sources disagree by more than `max_deviation_bps` — halt, do not average.
    DeviationBreaker = 4,
    /// Computed price outside the configured sane band.
    OutOfBand = 5,
}

#[contracttype]
#[derive(Clone)]
pub struct PriceData {
    /// Price in the oracle's base asset, at `decimals` precision.
    pub price: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct AdapterConfig {
    pub amm: Address,
    pub lp_token: Address,
    /// Independent feeds for the two legs — never the AMM itself.
    pub feed_a: Address,
    pub feed_b: Address,
    /// Beyond this age a feed is stale and borrowing halts.
    pub max_age_seconds: u64,
    /// Disagreement beyond this trips the breaker.
    pub max_deviation_bps: u32,
    /// Sanity band; outside it the adapter refuses rather than reporting.
    pub min_price: i128,
    pub max_price: i128,
}

#[contractclient(name = "LpOracleAdapterClient")]
pub trait LpOracleAdapterTrait {
    /// Blend-compatible price read. MUST fail closed: on a stale feed, a
    /// deviation trip or an out-of-band result, return an error rather than a
    /// best guess. Halting new borrows is always cheaper than lending against a
    /// bad price.
    ///
    /// IMPLEMENTER'S WARNING: if the underlying price entries can expire under
    /// Soroban state TTL, a MISSING price surfaces to callers as an untyped host
    /// trap (`VM call trapped: UnreachableCodeReached`) rather than a typed
    /// error, and nothing in the trace names the oracle. Integrators debugging a
    /// blanket revert should check price freshness first.
    fn lastprice(env: Env, asset: Address) -> Result<PriceData, Error>;

    fn decimals(env: Env) -> u32;
    fn base(env: Env) -> Address;
    fn get_config(env: Env) -> Result<AdapterConfig, Error>;

    /// True while the breaker is tripped. Consumers should refuse new borrows.
    fn is_halted(env: Env) -> bool;
}
