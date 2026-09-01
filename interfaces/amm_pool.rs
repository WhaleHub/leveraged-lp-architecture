//! Aquarius-style AMM pool — the subset actually consumed. Interface only.

#![no_std]

use soroban_sdk::{contractclient, contracttype, Address, Env, Vec};

#[contracttype]
#[derive(Clone)]
pub struct PoolInfo {
    /// StableSwap amplification. Can be changed on-chain via `ramp_a`, so do
    /// NOT hardcode it — read it, or avoid needing it via `estimate_swap`.
    pub a: u128,
    /// Fee in 1/10000 units (5 = 0.05%).
    pub fee: u32,
    pub n_tokens: u32,
}

#[contractclient(name = "AmmPoolClient")]
pub trait AmmPoolTrait {
    /// Token order is by ASCENDING contract address, not the order you listed
    /// them. Derive indices, never assume.
    fn get_tokens(env: Env) -> Vec<Address>;
    fn get_reserves(env: Env) -> Vec<u128>;
    fn get_total_shares(env: Env) -> u128;
    fn get_info(env: Env) -> PoolInfo;

    /// The pool's own quote. PREFER THIS over reimplementing the curve: it is
    /// exact, and it cannot drift when governance ramps `a`.
    fn estimate_swap(env: Env, in_idx: u32, out_idx: u32, in_amount: u128) -> u128;

    fn swap(env: Env, user: Address, in_idx: u32, out_idx: u32, in_amount: u128, out_min: u128)
        -> u128;

    /// Returns (actual amounts taken, shares minted). `min_shares` is the only
    /// thing standing between a mis-quoted zap and a drained flash loan — and
    /// note a 50/50 zap mints roughly HALF the borrowed amount in shares.
    fn deposit(env: Env, user: Address, desired_amounts: Vec<u128>, min_shares: u128)
        -> (Vec<u128>, u128);

    fn withdraw(env: Env, user: Address, share_amount: u128, min_amounts: Vec<u128>) -> Vec<u128>;
}
