#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::clock::Clock;
use anchor_lang::solana_program::sysvar::slot_hashes::SlotHashes;
use sha2::{Digest, Sha256};
use std::str::FromStr;

declare_id!("7abh1utsEzyXGPaA5ngBgnuj3PXRuzwtRM7ngEvUhrPG");

const OWNER: &str = "DGErPxhvoWWVVKZ6Jh47cGtHUstQBgtCHG3WEEig7LEZ";

#[program]
pub mod lottery {
    use super::*;

    #[access_control(check(&ctx))]
    pub fn generate_random(
        ctx: Context<Random>,
        uid: String,
    ) -> Result<()> {
        require!(uid.len() <= 12, Error::UidTooLong);

        let mut hasher = Sha256::new();
        hasher.update(&ctx.program_id.to_bytes());
        hasher.update(ctx.accounts.signer.key().as_ref());

        let clock = Clock::get()?;
        hasher.update(clock.slot.to_le_bytes());
        hasher.update(clock.unix_timestamp.to_le_bytes());

        hasher.update(uid.as_bytes());

        let slot_hashes = SlotHashes::from_account_info(&ctx.accounts.slot_hashes)?;
        let (_, recent_hash) = slot_hashes.iter().next().unwrap();
        hasher.update(recent_hash.as_ref());

        let seed: [u8; 32] = hasher.finalize().into();
        let full_range = u64::from_le_bytes(seed[..16].try_into().unwrap());
        let random_value = (full_range % 100000) as u32 + 1;

        let result = &mut ctx.accounts.result;
        result.uid = uid;
        result.value = random_value;
        result.timestamp = clock.unix_timestamp;

        Ok(())
    }
}

fn check(ctx: &Context<Random>) -> Result<()> {
    require_keys_eq!(ctx.accounts.signer.key(), OWNER.parse::<Pubkey>().unwrap(), Error::InvalidOwner);
    Ok(())
}

#[derive(Accounts)]
pub struct Random<'info> {
    #[account(
        mut,
        address = Pubkey::from_str(OWNER).unwrap(),
        constraint = signer.is_signer @ Error::InvalidSigner,
    )]
    pub signer: Signer<'info>,
    #[account(
        init_if_needed,
        payer = signer,
        space = 8 + LotteryResult::INIT_SPACE,
        seeds = [b"lottery_result", signer.key().as_ref()],
        bump
    )]
    pub result: Account<'info, LotteryResult>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
    /// CHECK:
    pub slot_hashes: AccountInfo<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct LotteryResult {
    #[max_len(12)]
    pub uid: String,
    pub value: u32,
    pub timestamp: i64,
}

#[error_code]
pub enum Error {
    #[msg("UID长度超过12字节")]
    UidTooLong,
    #[msg("账户所有者不匹配")]
    InvalidOwner,
    #[msg("未签名")]
    InvalidSigner
}