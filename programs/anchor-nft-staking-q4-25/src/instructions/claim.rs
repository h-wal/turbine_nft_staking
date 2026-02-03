use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

use crate::state::{StakeConfig, UserAccount};

// Claim instruction accounts
#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = reward_mint,
        associated_token::authority = user,
    )]
    pub rewards_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"config".as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
        seeds = [b"user".as_ref(), user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    #[account(
        mut,
        seeds = [b"rewards".as_ref(), config.key().as_ref()],
        bump = config.rewards_bump,
    )]
    pub reward_mint: Account<'info, Mint>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

// Claim instruction implementation
impl<'info> Claim<'info> {
    pub fn claim(&mut self) -> Result<()> {
        // Get points from user account
        let points = self.user_account.points as u64;

        // Signer seeds for minting
        let signer_seeds: &[&[&[u8]]] = &[&[b"config", &[self.config.bump]]];

        // Mint reward tokens to user's rewards account
        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.reward_mint.to_account_info(),
                    to: self.rewards_ata.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                // Signer seeds for minting
                signer_seeds,
            ),
            // Points to mint
            (points as u64).saturating_mul(10u64.pow(self.reward_mint.decimals as u32)),
        )?;

        // Reset points to 0
        self.user_account.points = 0;

        Ok(())
    }
}
