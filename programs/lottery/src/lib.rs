#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use std::str::FromStr;
use blake3;

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

        let mut seed = Vec::new();

        // 添加時間作為雜湊熵源
        // Add time as a hash entropy source
        // ハッシュのエントロピー源として時間を追加
        let clock = Clock::get()?;
        seed.extend_from_slice(&clock.slot.to_le_bytes());
        seed.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
        // 添加用戶ID作為哈希熵源
        // Add user ID as a hash entropy source
        // ハッシュのエントロピー源としてユーザーIDを追加
        seed.extend_from_slice(&uid.as_bytes());
        // 添加簽名者作為熵源
        // Add the signer as an entropy source
        // エントロピー源として署名者を追加
        seed.extend_from_slice(&ctx.accounts.signer.key().as_ref());
        // 添加結果接收者作為熵源
        // Add the result recipient as an entropy source
        // 結果の受信者をエントロピー源として追加
        seed.extend_from_slice(&ctx.accounts.result.key().as_ref());
        // 隨機選取一位平台用戶作為隨機的熵源
        // Randomly select a platform user as an entropy source for randomness
        // プラットフォームユーザーをランダムに抽出し、ランダム性のエントロピー源とする
        seed.extend_from_slice(&ctx.accounts.random_account.key().as_ref());

        // 第一輪雜湊
        // First-round hashing
        // 第一段階のハッシュ化
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);

        // 第二輪雜湊，消除熵源關聯性
        // Second-round hashing to eliminate entropy source correlation
        // 第二段階のハッシュ化により、エントロピー源の相関性を排除
        let mut blake = blake3::Hasher::new();
        blake.update(hasher.finalize().as_bytes());

        // 使用[拒絕抽樣法]消除取模偏誤，確保產生的隨機數在1-100000之間均勻分佈無偏誤。
        // Use [rejection sampling] to eliminate modulo bias and ensure that the generated random numbers are uniformly distributed between 1 and 100,000 without bias.
        // [リジェクションサンプリング]を使用して、モジュロバイアスを排除し、1〜100000の範囲で偏りなく一様分布する乱数を生成します。
        let max_safe = u64::MAX - (u64::MAX % 100000);
        let random_value;
        loop {
            let seed: [u8; 32] = blake.finalize().into();
            let raw = u64::from_le_bytes(seed[..8].try_into().unwrap());
            if raw < max_safe {
                random_value = (raw % 100000) as u32 + 1;
                break;
            }
            let clock = Clock::get()?;
            blake.update(&clock.slot.to_le_bytes());
            blake.update(&clock.unix_timestamp.to_le_bytes());
        }

        let result = &mut ctx.accounts.result;
        result.value = random_value;

        msg!("        Lottery User: {}", uid);
        msg!("Lottery Random Value: {}", random_value);
        msg!("        Lottery Time: {}", clock.unix_timestamp);

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
    /// CHECK: only use pubkey
    pub random_account: AccountInfo<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct LotteryResult {
    pub value: u32,
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
