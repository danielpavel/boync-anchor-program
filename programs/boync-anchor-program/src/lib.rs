use anchor_lang::prelude::*;

declare_id!("EkvnvRY2prU1sJpVLHBk5qsNMXBryaZeMVacRSE1pcZM");

pub const BOYNC_USER_PDA_SEED: &[u8] = b"user";

#[program]
pub mod boync_anchor_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String) -> Result<()> {
        if name.as_bytes().len() > 64 {
            return err!(ErrorCode::UserNameTooLong);
        }
        
        let user_data = &mut ctx.accounts.user_data;
        user_data.name = name;
        user_data.user = ctx.accounts.user.key();

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init, 
        payer = user,
        space = BOYNC_USER_DATA_LEN,
        seeds = [BOYNC_USER_PDA_SEED.as_ref(), user.key().as_ref()], 
        bump,
    )]
    pub user_data: Account<'info, BoyncUserData>,
    pub system_program: Program<'info, System>,
}

pub const BOYNC_USER_DATA_LEN: usize = 32 + (64 + 4) + 8;
#[account]
pub struct BoyncUserData {
    pub user: Pubkey,
    pub name: String, 
}

#[error_code]
pub enum ErrorCode {
    #[msg("User name can only be 64 chars long.")]
    UserNameTooLong,
}