//! Blend v2 pool — the subset actually consumed. Interface only.
//! Verified against `blend-contracts-v2` at the time of writing; treat the
//! upstream repository as authoritative.

#![no_std]

use soroban_sdk::{contractclient, contracttype, Address, Env, Map, Vec};

/// Request discriminants. Only 0-5 and 9 are used by this design.
pub const SUPPLY: u32 = 0;
pub const WITHDRAW: u32 = 1;
pub const SUPPLY_COLLATERAL: u32 = 2;
pub const WITHDRAW_COLLATERAL: u32 = 3;
pub const BORROW: u32 = 4;
pub const REPAY: u32 = 5;
pub const FILL_USER_LIQUIDATION_AUCTION: u32 = 6;
pub const FILL_BAD_DEBT_AUCTION: u32 = 7;
pub const FILL_INTEREST_AUCTION: u32 = 8;
pub const DELETE_LIQUIDATION_AUCTION: u32 = 9;

#[contracttype]
#[derive(Clone)]
pub struct Request {
    pub request_type: u32,
    pub address: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct FlashLoan {
    /// The receiver. MUST NOT be the initiator — Soroban forbids re-entry.
    pub contract: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Positions {
    /// reserve_index -> bToken amount
    pub collateral: Map<u32, i128>,
    /// reserve_index -> dToken amount
    pub liabilities: Map<u32, i128>,
    pub supply: Map<u32, i128>,
}

#[contractclient(name = "BlendPoolClient")]
pub trait BlendPoolTrait {
    /// Execution order, which drives the whole design:
    ///   1. add the borrowed amount as a REAL liability to `from`
    ///   2. transfer the asset to `flash_loan.contract`
    ///   3. call `exec_op(from, asset, amount, 0)` on it
    ///   4. process `requests` for `from` via transfer_from / ALLOWANCE
    ///   5. run the health check ONCE
    ///
    /// Step 4 is why the initiator must approve the pool before the loan.
    /// Step 5 is why the position is never observably unhealthy mid-build.
    fn flash_loan(env: Env, from: Address, flash_loan: FlashLoan, requests: Vec<Request>)
        -> Positions;

    /// Non-flash path. `Repay` pulls via `transfer` with `spender`, which an
    /// allowance does NOT cover — a contract spender must authorize that
    /// sub-invocation itself.
    fn submit(env: Env, from: Address, spender: Address, to: Address, requests: Vec<Request>)
        -> Positions;

    /// Source of truth for share-based accounting. Never mirror this in a
    /// local ledger: an internal copy desyncs the moment a liquidation lands.
    fn get_positions(env: Env, address: Address) -> Positions;
}
