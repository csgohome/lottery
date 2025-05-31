use anchor_lang::prelude::*;

declare_id!("3a9YqNvBrF1qXFvZvfS72ZqyxGyVX3Up9bNPQQrtbCTp");

#[program]
pub mod lottery {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
