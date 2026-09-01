//! Zapper — the flash-loan receiver. Interface only, no implementation.
//!
//! WHY THIS CONTRACT EXISTS
//! -----------------------
//! It would be simpler for the vault to receive its own flash loan. It does not
//! work. The vault is still on the call stack inside `open_position` when the
//! pool calls back, and the Soroban host rejects the re-entry:
//!
//!     Error(Context, InvalidAction)
//!       -> "Contract re-entry is not allowed"
//!
//! Moving `exec_op` onto a contract that is NOT already on the stack is the
//! entire fix. Everything else about the request stack is unchanged.
//!
//! The zapper holds no user funds. `exec_op` is guarded by a transient flag set
//! by `prepare`, so it is only callable by the pool inside a loan the vault
//! staged — never standalone.

#![no_std]

use soroban_sdk::{contractclient, contracterror, contracttype, Address, BytesN, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// `exec_op` fired without a staged `prepare`.
    NoPending = 3,
    /// The pool delivered an asset this zapper was not staged for.
    AssetMismatch = 4,
    /// Caller is not the configured pool.
    Unauthorized = 5,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    /// The only address permitted to call `exec_op`.
    pub blend_pool: Address,
    /// The only vault permitted to `prepare`. Pinning this is what stops a
    /// third party staging a zap that spends the pool's flash loan.
    pub vault: Address,
    pub amm: Address,
    pub lp_token: Address,
    pub borrow_asset: Address,
    pub pair_token: Address,
    /// Token indices within the AMM (Aquarius sorts by ascending address).
    pub borrow_idx: u32,
    pub pair_idx: u32,
}

#[contractclient(name = "ZapperClient")]
pub trait ZapperTrait {
    fn initialize(
        env: Env,
        vault: Address,
        blend_pool: Address,
        amm: Address,
        lp_token: Address,
        borrow_asset: Address,
        pair_token: Address,
        borrow_idx: u32,
        pair_idx: u32,
    ) -> Result<(), Error>;

    /// Stage the next zap. Callable only by the configured vault, immediately
    /// before it initiates the flash loan.
    fn prepare(env: Env, borrow_amount: i128, min_lp_out: i128, min_pair_out: i128)
        -> Result<(), Error>;

    /// Flash-loan callback. Invoked by the pool with the borrowed asset already
    /// transferred in. Must:
    ///   1. swap half the borrowed asset for the pair leg (>= min_pair_out)
    ///   2. deposit both legs into the AMM (>= min_lp_out shares)
    ///   3. transfer the minted LP to the vault, which has already approved the
    ///      pool to pull it via `transfer_from`
    ///
    /// `fee` is part of the Blend receiver signature and is 0 in current usage.
    fn exec_op(env: Env, caller: Address, token: Address, amount: i128, fee: i128)
        -> Result<(), Error>;

    fn get_config(env: Env) -> Result<Config, Error>;
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>;
}
