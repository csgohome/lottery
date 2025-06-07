#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
use blake3;

declare_id!("7abh1utsEzyXGPaA5ngBgnuj3PXRuzwtRM7ngEvUhrPG");

// 随机数范围1-？
const MAX_RANDOM:u64 = 100000;

// 使用[拒絕抽樣法]消除取模偏誤，確保產生的隨機數在1-100000之間均勻分佈無偏誤。
// Use [rejection sampling] to eliminate modulo bias and ensure that the generated random numbers are uniformly distributed between 1 and 100,000 without bias.
// [リジェクションサンプリング]を使用して、モジュロバイアスを排除し、1〜100000の範囲で偏りなく一様分布する乱数を生成します。
const MAX_SAFE: u64 = u64::MAX - (u64::MAX % MAX_RANDOM);
fn rejection_sampling(raw: u64) -> Option<u32> {
    if raw < MAX_SAFE {
        Some((raw % MAX_RANDOM) as u32 + 1)
    } else {
        None
    }
}

#[program]
pub mod lottery {
    use super::*;

    pub fn generate_random<'a>(
        ctx: Context<Random>,
        order_id: String,
        count: u8,
    ) -> Result<()> {
        require!(order_id.len() <= 12, Error::OrderIdTooLong);
        require!(count <= 200,Error::InvalidCount);

        // 添加訂單ID作為雜湊熵源
        // Incorporate the order ID as an entropy source for hashing
        // ハッシュのエントロピー源として注文IDを追加
        let order_bytes = order_id.as_bytes();
        let count_bytes = count.to_le_bytes();
        // 添加時間作為雜湊熵源
        // Add time as a hash entropy source
        // ハッシュのエントロピー源として時間を追加
        let clock = Clock::get()?;
        let slot_bytes = clock.slot.to_le_bytes();
        let timestamp_bytes = clock.unix_timestamp.to_le_bytes();
        // 添加簽名者作為熵源
        // Add the signer as an entropy source
        // エントロピー源として署名者を追加
        let signer_bytes = ctx.accounts.signer.key().to_bytes();
        // 隨機選取一位平台用戶作為隨機的熵源
        // Randomly select a platform user as an entropy source for randomness
        // プラットフォームユーザーをランダムに抽出し、ランダム性のエントロピー源とする
        let random_bytes = ctx.accounts.random_account.key().to_bytes();
        
        let mut data = Vec::with_capacity(order_bytes.len() + count_bytes.len() + slot_bytes.len() + timestamp_bytes.len() + signer_bytes.len() + random_bytes.len());
        data.extend_from_slice(order_bytes);
        data.extend_from_slice(&count_bytes);
        data.extend_from_slice(&slot_bytes);
        data.extend_from_slice(&timestamp_bytes);
        data.extend_from_slice(&signer_bytes);
        data.extend_from_slice(&random_bytes);
        
        // 第一輪雜湊
        // First-round hashing
        // 第一段階のハッシュ化
        let mut hasher = blake3::Hasher::new();
        hasher.update(&data);
        let intermediate = hasher.finalize();

        let mut arr: Vec<u32> = Vec::with_capacity(count as usize);
        let mut iteration_count:i32 = 0;
        while arr.len() < count as usize {
            iteration_count += 1;
            // 第二輪雜湊，消除熵源關聯性
            // Second-round hashing to eliminate entropy source correlation
            // 第二段階のハッシュ化により、エントロピー源の相関性を排除
            hasher.reset(); // 重置哈希器狀態
            hasher.update(intermediate.as_bytes());
            hasher.update(&iteration_count.to_le_bytes());
            let batch_hash = hasher.finalize();
            let bytes = batch_hash.as_bytes()[..8].try_into().unwrap();
            let raw = u64::from_le_bytes(bytes);
            match rejection_sampling(raw) {
                Some(random_value) => {
                    arr.push(random_value);
                }
                None => {
                    msg!("Rejection sampling failed for raw value: {}", raw);
                    break
                }
            }
        }

        msg!("ORDER ID    :{}", order_id);
        msg!("RANDOM VALUE:{:?}", arr);
        msg!("RANDOM TIME :{}", clock.unix_timestamp);

        emit!(RandomResult {
            order_id,
            value: arr,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Random<'info> {
    #[account(
        mut,
        constraint = signer.is_signer @ Error::InvalidSigner,
    )]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
    /// CHECK: readonly
    pub random_account: AccountInfo<'info>,
}

#[event]
pub struct RandomResult {
    pub order_id: String,
    pub value: Vec<u32>,
    pub timestamp: i64,
}

#[error_code]
pub enum Error {
    #[msg("Order id length exceeds 12 bytes")]
    OrderIdTooLong,
    #[msg("Get up to 200 random numbers at one time")]
    InvalidCount,
    #[msg("Not signed")]
    InvalidSigner,
}
