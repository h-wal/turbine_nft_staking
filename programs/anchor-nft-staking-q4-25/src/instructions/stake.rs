use anchor_lang::prelude::*;
use mpl_core::{
    instructions::AddPluginV1CpiBuilder,
    types::{FreezeDelegate, Plugin, PluginAuthority},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    errors::StakeError,
    state::{StakeAccount, StakeConfig, UserAccount},
};

// Stake instruction accounts
#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Validated by Core program CPI
    #[account(mut)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Validated by Core program CPI
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,

    // Stake account
    #[account(
        init,
        payer = user,
        seeds = [b"stake", config.key().as_ref(), asset.key().as_ref()],
        bump,
        space = StakeAccount::DISCRIMINATOR.len() + StakeAccount::INIT_SPACE,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    // Stake config
    #[account(
        seeds = [b"config".as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, StakeConfig>,

    // User account
    #[account(
        mut,
        seeds = [b"user".as_ref(), user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    // Core program
    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Metaplex Core program
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    // Stake instruction implementation
    pub fn stake(&mut self, bumps: &StakeBumps) -> Result<()> {
        // Check if user has reached max stake
        require!(
            self.user_account.amount_staked < self.config.max_stake,
            StakeError::MaxStakeReached
        );

        // Add FreezeDelegate plugin with stake account as authority
        AddPluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(None)
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::Address {
                address: self.stake_account.key(),
            })
            .invoke()?;

        // Initialize stake account with stake account information
        self.stake_account.set_inner(StakeAccount {
            owner: self.user.key(),
            mint: self.asset.key(),
            staked_at: Clock::get()?.unix_timestamp,
            bump: bumps.stake_account,
        });

        // Increment amount staked by 1
        self.user_account.amount_staked += 1;

        Ok(())
    }
}