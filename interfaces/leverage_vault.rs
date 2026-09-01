//! Leverage vault — interface only, no implementation.
//!
//! The vault is the Blend `from`: it owns ONE lending position and tracks each
//! depositor as shares of it. It is deliberately NOT the flash-loan receiver —
//! see `zapper.rs` for why that split is mandatory rather than stylistic.
//!
//! Accounting invariant: a user's position is never stored as an amount. It is
//! derived on read from live pool state, so interest and liquidation flow into
//! the share price with nothing to reconcile.
//!
//!     user_collateral = collateral_shares / total_collateral_shares
//!                       * pool.get_positions(vault).collateral[lp_reserve_index]
//!
//! Consequence, stated plainly: a voluntary exit takes exactly the leaver's
//! share and leaves others unchanged, but a LIQUIDATION is shared pro-rata
//! across every depositor. This interface does not isolate liquidation losses.

#![no_std]

use soroban_sdk::{contractclient, contracterror, contracttype, Address, BytesN, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidAmount = 3,
    /// Requested leverage exceeds `max_leverage_bps`.
    ///
    /// NOTE: `#4` is also `SlippageExceeded` in the AMM. When a revert surfaces
    /// as `Error(Contract, #4)`, read to the DEEPEST diagnostic event before
    /// concluding which contract rejected it.
    LeverageTooHigh = 4,
    NoPosition = 7,
    InsufficientCollateral = 8,
    InsufficientDebt = 9,
    /// The flash loan produced no measurable collateral delta.
    NothingMinted = 10,
}

#[contracttype]
#[derive(Clone)]
pub struct Config {
    pub admin: Address,
    /// Flash-loan receiver. Must NOT be this contract (Soroban re-entrancy).
    pub zapper: Address,
    pub blend_pool: Address,
    /// The AMM share token used as collateral.
    pub lp_token: Address,
    pub borrow_asset: Address,
    pub lp_reserve_index: u32,
    pub borrow_reserve_index: u32,
    /// Should sit at ~92% of the pool's own boundary,
    /// `1 / (1 - c_factor * l_factor)`. A cap above the boundary is not a cap.
    pub max_leverage_bps: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct UserShares {
    pub collateral_shares: i128,
    pub debt_shares: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct UserPosition {
    /// Derived live, never stored.
    pub collateral_lp: i128,
    pub debt: i128,
}

#[contractclient(name = "LeverageVaultClient")]
pub trait LeverageVaultTrait {
    fn initialize(
        env: Env,
        admin: Address,
        zapper: Address,
        blend_pool: Address,
        lp_token: Address,
        borrow_asset: Address,
        lp_reserve_index: u32,
        borrow_reserve_index: u32,
        max_leverage_bps: u32,
    ) -> Result<(), Error>;

    /// Open or add to a leveraged position, atomically.
    ///
    /// Ordering matters and is not optional:
    ///   1. snapshot the vault's pool position (bToken/dToken)
    ///   2. pull the user's LP
    ///   3. approve the pool for equity + expected zap output — `SupplyCollateral`
    ///      pulls by `transfer_from`, so the approval must precede the loan
    ///   4. stage the zap on the zapper
    ///   5. `flash_loan` with the zapper as receiver
    ///   6. mint shares from the MEASURED delta, not the requested amounts
    ///
    /// `min_lp_out` must come from a live-reserves quote. A 50/50 zap mints
    /// roughly HALF the borrowed amount in LP; a floor near `borrow_amount`
    /// can never be met and every open reverts.
    fn open_position(
        env: Env,
        user: Address,
        collateral_lp_amount: i128,
        borrow_amount: i128,
        min_lp_out: i128,
        min_pair_out: i128,
    ) -> Result<(), Error>;

    /// Deleverage: repay debt (pulled from `user`) and withdraw LP collateral.
    ///
    /// The pool pulls the repay amount by `transfer` with the vault as SPENDER,
    /// which an allowance does not satisfy — the implementation must
    /// `authorize_as_current_contract` that sub-invocation.
    ///
    /// v1 requires the user to bring the repay asset; a flash-loan self-closing
    /// path is the natural follow-up.
    fn repay_and_withdraw(
        env: Env,
        user: Address,
        repay_amount: i128,
        withdraw_lp_amount: i128,
        min_lp_out_to_user: i128,
    ) -> Result<(), Error>;

    /// Delete a stale liquidation auction on the vault's own position, valid
    /// once it is healthy again. Permissionless: `from` is the vault, so it can
    /// only ever clear its own auction.
    fn clear_auction(env: Env) -> Result<(), Error>;

    // ---- views: all derived from live pool state ----
    fn get_config(env: Env) -> Result<Config, Error>;
    fn get_position(env: Env, user: Address) -> UserPosition;
    fn get_user_shares(env: Env, user: Address) -> UserShares;
    /// (total collateral, total debt) of the vault's live pool position.
    fn get_totals(env: Env) -> (i128, i128);
    fn get_share_totals(env: Env) -> (i128, i128);

    // ---- admin ----
    fn set_admin(env: Env, new_admin: Address) -> Result<(), Error>;
    /// In-place upgrade keeps the address and all share state stable, so a fix
    /// never migrates user positions.
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>;
}
