use anchor_lang::{
  prelude::*,
  solana_program::{
    instruction::{AccountMeta, Instruction},
  },
};

use mpl_token_metadata::instruction::{MetadataInstruction, TransferArgs};

pub fn build_mpl_metadata_transfer<'info>(
    token: Pubkey,
    token_owner: Pubkey,
    destination: Pubkey,
    destination_owner: Pubkey,
    mint: Pubkey,
    metadata: Pubkey,
    edition: Option<Pubkey>,
    owner_token_record: Option<Pubkey>,
    destination_token_record: Option<Pubkey>,
    authority: Pubkey,
    payer: Pubkey,
    system_program: Pubkey,
    sysvar_instructions: Pubkey,
    spl_token_program: Pubkey,
    spl_ata_program: Pubkey,
    authorization_rules: Option<Pubkey>,
    authorization_rules_program: Option<Pubkey>,
) -> anchor_lang::solana_program::instruction::Instruction {
    let args = TransferArgs::V1 {
        authorization_data: None,
        amount: 1,
    };

    let mut accounts = vec![
        AccountMeta::new(token, false),
        AccountMeta::new_readonly(token_owner, false),
        AccountMeta::new(destination, false),
        AccountMeta::new_readonly(destination_owner, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(metadata, false),
        AccountMeta::new_readonly(edition.unwrap_or(crate::ID), false),
        if let Some(owner_token_record) = owner_token_record {
            AccountMeta::new(owner_token_record, false)
        } else {
            AccountMeta::new_readonly(crate::ID, false)
        },
        if let Some(destination_token_record) = destination_token_record {
            AccountMeta::new(destination_token_record, false)
        } else {
            AccountMeta::new_readonly(crate::ID, false)
        },
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(system_program, false),
        AccountMeta::new_readonly(sysvar_instructions, false),
        AccountMeta::new_readonly(spl_token_program, false),
        AccountMeta::new_readonly(spl_ata_program, false),
    ];
    // Optional authorization rules accounts
    if let Some(rules) = &authorization_rules {
        accounts.push(AccountMeta::new_readonly(
            authorization_rules_program.unwrap_or(crate::ID),
            false,
        ));
        accounts.push(AccountMeta::new_readonly(*rules, false));
    } else {
        accounts.push(AccountMeta::new_readonly(crate::ID, false));
        accounts.push(AccountMeta::new_readonly(crate::ID, false));
    }

    Instruction {
        program_id: mpl_token_metadata::ID,
        accounts,
        data: MetadataInstruction::Transfer(args.clone())
            .try_to_vec()
            .unwrap(),
    }
}

