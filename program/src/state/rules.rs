use crate::{
    error::RuleSetError,
    payload::{ParsedPayload, PayloadField, PayloadKey},
    pda::FREQ_PDA,
    utils::assert_derivation,
};
use serde::{Deserialize, Serialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey, sysvar::Sysvar,
};
use std::collections::HashMap;

use super::{FrequencyAccount, SolanaAccount};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum Rule {
    All {
        rules: Vec<Rule>,
    },
    Any {
        rules: Vec<Rule>,
    },
    Not {
        rule: Box<Rule>,
    },
    AdditionalSigner {
        account: Pubkey,
    },
    PubkeyMatch {
        pubkey: Pubkey,
        field: PayloadKey,
    },
    DerivedKeyMatch {
        account: Pubkey,
        field: PayloadKey,
    },
    ProgramOwned {
        program: Pubkey,
        field: PayloadKey,
    },
    Amount {
        amount: u64,
    },
    Frequency {
        freq_name: String,
        freq_account: Pubkey,
    },
    PubkeyTreeMatch {
        root: [u8; 32],
        field: PayloadKey,
    },
}

impl Rule {
    pub fn validate(
        &self,
        accounts: &HashMap<Pubkey, &AccountInfo>,
        payload: &Vec<PayloadField>,
    ) -> ProgramResult {
        let mut parsed_payload = ParsedPayload::default();
        for field in payload {
            match field {
                PayloadField::Target(target) => parsed_payload.target = Some(target.clone()),
                PayloadField::Holder(holder) => parsed_payload.holder = Some(holder.clone()),
                PayloadField::Authority(authority) => {
                    parsed_payload.authority = Some(authority.clone())
                }
                PayloadField::Amount(amount) => parsed_payload.amount = Some(*amount),
            }
        }

        let (status, rollup_err) = self.ll_validate(accounts, &parsed_payload);

        if status {
            ProgramResult::Ok(())
        } else {
            ProgramResult::Err(rollup_err.into())
        }
    }

    pub fn ll_validate(
        &self,
        accounts: &HashMap<Pubkey, &AccountInfo>,
        payload: &ParsedPayload,
    ) -> (bool, RuleSetError) {
        match self {
            Rule::All { rules } => {
                msg!("Validating All");
                let mut last = self.to_error();
                for rule in rules {
                    last = rule.to_error();
                    let result = rule.ll_validate(accounts, payload);
                    if !result.0 {
                        return result;
                    }
                }
                (true, last)
            }
            Rule::Any { rules } => {
                msg!("Validating Any");
                let mut last = self.to_error();
                for rule in rules {
                    last = rule.to_error();
                    let result = rule.ll_validate(accounts, payload);
                    if result.0 {
                        return result;
                    }
                }
                (false, last)
            }
            Rule::Not { rule } => {
                let result = rule.ll_validate(accounts, payload);
                (!result.0, result.1)
            }
            Rule::AdditionalSigner { account } => {
                msg!("Validating AdditionalSigner");
                if let Some(signer) = accounts.get(account) {
                    (signer.is_signer, self.to_error())
                } else {
                    (false, self.to_error())
                }
            }
            Rule::PubkeyMatch { pubkey, field } => {
                msg!("Validating PubkeyMatch");

                let key = match payload.get_pubkey(*field) {
                    Some(pubkey) => pubkey,
                    _ => return (false, self.to_error()),
                };

                if key == *pubkey {
                    (true, self.to_error())
                } else {
                    (false, self.to_error())
                }
            }
            Rule::DerivedKeyMatch { account, field } => {
                msg!("Validating DerivedKeyMatch");

                let seeds = match payload.get_seeds(*field) {
                    Some(seeds) => seeds,
                    _ => return (false, self.to_error()),
                };

                let vec_of_slices = seeds
                    .seeds
                    .iter()
                    .map(Vec::as_slice)
                    .collect::<Vec<&[u8]>>();
                let seeds = &vec_of_slices[..];
                if let Ok(_bump) = assert_derivation(&crate::id(), account, seeds) {
                    (true, self.to_error())
                } else {
                    (false, self.to_error())
                }
            }
            Rule::ProgramOwned { program, field } => {
                msg!("Validating ProgramOwned");

                let key = match payload.get_pubkey(*field) {
                    Some(pubkey) => pubkey,
                    _ => return (false, self.to_error()),
                };

                if let Some(account) = accounts.get(&key) {
                    if *account.owner == *program {
                        return (true, self.to_error());
                    }
                }

                (false, self.to_error())
            }
            Rule::Amount { amount } => {
                msg!("Validating Amount");
                if let Some(payload_amount) = &payload.amount {
                    if amount == payload_amount {
                        (true, self.to_error())
                    } else {
                        (false, self.to_error())
                    }
                } else {
                    (false, self.to_error())
                }
            }
            Rule::Frequency {
                freq_name: _,
                freq_account,
            } => {
                msg!("Validating Frequency");
                // Deserialize the frequency account
                if let Some(account) = accounts.get(freq_account) {
                    if let Ok(current_time) = solana_program::clock::Clock::get() {
                        let freq_account = FrequencyAccount::from_account_info(account);
                        if let Ok(freq_account) = freq_account {
                            // Grab the current time
                            // Compare  last time + period to current time
                            if let Some(freq_check) =
                                freq_account.last_update.checked_add(freq_account.period)
                            {
                                if freq_check < current_time.unix_timestamp {
                                    (true, self.to_error())
                                } else {
                                    (false, self.to_error())
                                }
                            } else {
                                (false, RuleSetError::NumericalOverflow)
                            }
                        } else {
                            (false, self.to_error())
                        }
                    } else {
                        (false, self.to_error())
                    }
                } else {
                    (false, self.to_error())
                }
            }
            Rule::PubkeyTreeMatch { root, field } => {
                msg!("Validating PubkeyTreeMatch");

                let merkle_proof = match payload.get_merkle_proof(*field) {
                    Some(merkle_proof) => merkle_proof,
                    _ => return (false, self.to_error()),
                };

                let mut computed_hash = merkle_proof.leaf;
                for proof_element in merkle_proof.proof.iter() {
                    if computed_hash <= *proof_element {
                        // Hash(current computed hash + current element of the proof)
                        computed_hash = solana_program::keccak::hashv(&[
                            &[0x01],
                            &computed_hash,
                            proof_element,
                        ])
                        .0;
                    } else {
                        // Hash(current element of the proof + current computed hash)
                        computed_hash = solana_program::keccak::hashv(&[
                            &[0x01],
                            proof_element,
                            &computed_hash,
                        ])
                        .0;
                    }
                }
                // Check if the computed hash (root) is equal to the provided root
                if computed_hash == *root {
                    (true, self.to_error())
                } else {
                    (false, self.to_error())
                }
            }
        }
    }

    pub fn assert_rule_pda_derivations(
        &self,
        payer: &Pubkey,
        rule_set_name: &String,
    ) -> ProgramResult {
        match self {
            Rule::All { rules } => {
                for rule in rules {
                    rule.assert_rule_pda_derivations(payer, rule_set_name)?;
                }
                Ok(())
            }
            Rule::Any { rules } => {
                let mut error: Option<ProgramResult> = None;
                for rule in rules {
                    match rule.assert_rule_pda_derivations(payer, rule_set_name) {
                        Ok(_) => return Ok(()),
                        Err(e) => error = Some(Err(e)),
                    }
                }
                error.unwrap_or_else(|| Err(RuleSetError::DataTypeMismatch.into()))
            }
            Rule::Frequency {
                freq_name,
                freq_account,
            } => {
                msg!("Assert Frequency PDA deriviation");
                // Check Frequency account info derivation.
                let _bump = assert_derivation(
                    &crate::id(),
                    freq_account,
                    &[
                        FREQ_PDA.as_bytes(),
                        payer.as_ref(),
                        rule_set_name.as_bytes(),
                        freq_name.as_bytes(),
                    ],
                )?;

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn to_error(&self) -> RuleSetError {
        match self {
            Rule::AdditionalSigner { .. } => RuleSetError::AdditionalSignerCheckFailed,
            Rule::PubkeyMatch { .. } => RuleSetError::PubkeyMatchCheckFailed,
            Rule::DerivedKeyMatch { .. } => RuleSetError::DerivedKeyMatchCheckFailed,
            Rule::ProgramOwned { .. } => RuleSetError::ProgramOwnedCheckFailed,
            Rule::Amount { .. } => RuleSetError::AmountCheckFailed,
            Rule::Frequency { .. } => RuleSetError::FrequencyCheckFailed,
            Rule::PubkeyTreeMatch { .. } => RuleSetError::PubkeyTreeMatchCheckFailed,
            _ => RuleSetError::NotImplemented,
        }
    }
}
