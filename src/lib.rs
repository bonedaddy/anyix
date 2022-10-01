use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult};

/// Helper function to handle unpacking instruction data
/// and executing the contained instructions. At the moment
/// this function is not capable of handling signed CPI invocation
/// and will carry through signing permissions from the transaction itself
pub fn handle_anyix<'info>(
    program_id: Pubkey,
    accounts: &[AccountInfo<'info>],
    data: &[u8],
) -> ProgramResult {
    let arb_ix = AnyIx::unpack(data).unwrap();
    let AnyIx {
        num_instructions,
        instruction_data_sizes: _,
        instruction_datas,
        instruction_account_counts,
    } = arb_ix;
    let mut offset = 0;
    for idx in 0..num_instructions {
        let accounts =
            &accounts[offset as usize..instruction_account_counts[idx as usize] as usize];
        offset += instruction_account_counts[idx as usize];
        let program_account = &accounts[0];
        if program_id.eq(program_account.key) {
            panic!("self invocation not allowed");
        }
        solana_program::program::invoke(
            &Instruction {
                program_id: *program_account.key,
                accounts: accounts[1..]
                    .iter()
                    .map(|account| {
                        if account.is_writable {
                            AccountMeta::new(*account.key, account.is_signer)
                        } else {
                            AccountMeta::new_readonly(*account.key, account.is_signer)
                        }
                    })
                    .collect(),
                data: instruction_datas[idx as usize].clone(),
            },
            accounts,
        )?;
    }
    Ok(())
}

/// encodes a set of instructions into the AnyIx format
pub fn encode_instructions(ixs: &[Instruction]) -> AnyIx {
    let num_instructions = ixs.len();

    let ix_data_sizes: Vec<u8> = ixs
        .iter()
        .map(|ix| ix.data.len().try_into().unwrap())
        .collect();
    let ix_datas = ixs.iter().map(|ix| ix.data.clone()).collect::<Vec<_>>();
    let ix_account_counts: Vec<u8> = ixs
        .iter()
        .map(|ix| ix.accounts.len().try_into().unwrap())
        .collect::<Vec<_>>();
    AnyIx {
        num_instructions: num_instructions as u8,
        instruction_account_counts: ix_account_counts,
        instruction_data_sizes: ix_data_sizes,
        instruction_datas: ix_datas,
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AnyIx {
    /// the total number of individual instructions
    pub num_instructions: u8,
    pub instruction_data_sizes: Vec<u8>,
    /// the number of accounts to use for a single instruction
    /// for example of this field is set to vec![10, 5], then the first instruction
    /// uses 10 accounts, with the second instruction using 5 accounts
    pub instruction_account_counts: Vec<u8>,
    /// a vector of vectors, where each element is the instruction data
    /// to pass for instruction_datas[N]
    pub instruction_datas: Vec<Vec<u8>>,
}

impl AnyIx {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        if input.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let (num_instructions, data) = AnyIx::unpack_u8_slice(&input[0..], 1)?;
        let (instruction_data_sizes, data) =
            AnyIx::unpack_u8_slice(data, num_instructions[0] as usize)?;
        let (instruction_account_counts, mut data) =
            AnyIx::unpack_u8_slice(data, num_instructions[0] as usize)?;
        let mut instruction_datas = Vec::with_capacity(30);
        for data_size in instruction_data_sizes {
            let (ix_data, data2) = data.split_at(*data_size as usize);
            data = data2;
            instruction_datas.push(ix_data.to_vec());
        }
        Ok(AnyIx {
            num_instructions: num_instructions[0],
            instruction_data_sizes: instruction_data_sizes.to_vec(),
            instruction_account_counts: instruction_account_counts.to_vec(),
            instruction_datas: instruction_datas.to_vec(),
        })
    }
    pub fn pack(&self) -> Result<Vec<u8>, ProgramError> {
        let mut datas = Vec::with_capacity(std::mem::size_of_val(self));
        datas.push(self.num_instructions);
        datas.extend_from_slice(&self.instruction_data_sizes[..]);
        datas.extend_from_slice(&self.instruction_account_counts[..]);
        for ix_data in self.instruction_datas.iter() {
            datas.extend_from_slice(ix_data);
        }
        Ok(datas)
    }
    // returns a slice of of `count` values
    fn unpack_u8_slice(input: &[u8], count: usize) -> Result<(&[u8], &[u8]), ProgramError> {
        Ok(input.split_at(count))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use solana_program::pubkey::Pubkey;
    #[test]
    fn test_any_ix() {
        {
            let ix_1 = spl_token::instruction::transfer(
                &spl_token::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
                100,
            )
            .unwrap();
            let ix_2 = spl_token::instruction::transfer(
                &spl_token::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
                200,
            )
            .unwrap();
            let ix_3 = spl_token::instruction::transfer(
                &spl_token::id(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                &[],
                300,
            )
            .unwrap();
            let want_arb_any = AnyIx {
                num_instructions: 3,
                instruction_data_sizes: vec![
                    ix_1.data.len() as u8,
                    ix_2.data.len() as u8,
                    ix_3.data.len() as u8,
                ],
                instruction_datas: vec![ix_1.data.clone(), ix_2.data.clone(), ix_3.data.clone()],
                instruction_account_counts: vec![3, 3, 3],
            };
            let want_arb_any_data = want_arb_any.pack().unwrap();
            let got_arb_aby = AnyIx::unpack(&want_arb_any_data).unwrap();
            assert_eq!(got_arb_aby, want_arb_any);

            let encoded_ix = encode_instructions(&[ix_1, ix_2, ix_3]);

            assert_eq!(want_arb_any, encoded_ix);
        }
    }
}
