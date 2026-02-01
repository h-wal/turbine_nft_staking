use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin, PluginType},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    errors::StakeError,
    state::{StakeAccount, StakeConfig, UserAccount},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
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
        mut,
        close = user,
        seeds = [b"stake", config.key().as_ref(), asset.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == user.key() @ StakeError::NotOwner,
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

    #[account(address = CORE_PROGRAM_ID)]
    /// CHECK: Metaplex Core program
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    // Unstake instruction implementation
    pub fn unstake(&mut self) -> Result<()> {
        // Calculate time elapsed since staked
        let time_elapsed =
            ((Clock::get()?.unix_timestamp - self.stake_account.staked_at) / 86400) as u32;

        // Check if freeze period has passed
        require!(
            time_elapsed >= self.config.freeze_period,
            StakeError::FreezePeriodNotPassed
        );

        // Calculate and award points based on time elapsed and points per stake
        self.user_account.points += time_elapsed * self.config.points_per_stake as u32;

        // Signer seeds for stake account PDA
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"stake",
            self.config.to_account_info().key.as_ref(),
            self.asset.to_account_info().key.as_ref(),
            &[self.stake_account.bump],
        ]];

        // Update FreezeDelegate plugin to false
        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.stake_account.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;

        // Remove FreezeDelegate plugin
        RemovePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(None)
            .system_program(&self.system_program.to_account_info())
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        // Decrement amount staked by 1
        self.user_account.amount_staked -= 1;

        Ok(())
    }
}
