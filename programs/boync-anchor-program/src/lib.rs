use anchor_lang::prelude::*;

declare_id!("hqgrvUepLLhFbXCb8woduWM62ps5rqap3TmPHbpuK11");

// pub const BOYNC_USER_PDA_SEED: &[u8] = b"user";
pub const BOYNC_AUCTION_PDA_SEED: &[u8] = b"auction";

#[program]
pub mod boync_anchor_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String, token: Pubkey) -> Result<()> {
        msg!("Message from instruction");

        if name.as_bytes().len() > 64 {
            return err!(ErrorCode::UserNameTooLong);
        }
        
        let user_data = &mut ctx.accounts.user_data;

        user_data.name = name;
        user_data.token = token;
        user_data.authority = ctx.accounts.authority.key();

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init, 
        payer = authority,
        space = BOYNC_AUCTION_DATA_LEN,
        seeds = [BOYNC_AUCTION_PDA_SEED.as_ref(), authority.key().as_ref()], 
        bump,
    )]
    pub user_data: Account<'info, BoyncAuctionData>,
    pub system_program: Program<'info, System>,
}

// #[derive(Accounts)]
// pub struct Initialize<'info> {
//     #[account(mut)]
//     pub user: Signer<'info>,
//     #[account(
//         init, 
//         payer = user,
//         space = BOYNC_USER_DATA_LEN,
//         seeds = [BOYNC_USER_PDA_SEED.as_ref(), user.key().as_ref()], 
//         bump,
//     )]
//     pub user_data: Account<'info, BoyncUserData>,
//     pub system_program: Program<'info, System>,
// }

// pub const BOYNC_USER_DATA_LEN: usize = 32 + (64 + 4) + 8;
// #[account]
// pub struct BoyncUserData {
//     pub user: Pubkey,
//     pub name: String, 
// }

pub const BOYNC_AUCTION_DATA_LEN: usize = 32 + 32 + (64 + 4) + 8;
#[account]
pub struct BoyncAuctionData {
    pub authority: Pubkey,
    pub token: Pubkey,
    pub name: String, 
}

#[error_code]
pub enum ErrorCode {
    #[msg("User name can only be 64 chars long.")]
    UserNameTooLong,
}