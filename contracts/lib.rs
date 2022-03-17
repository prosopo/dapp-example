// Copyright (C) 2021-2022 Prosopo (UK) Ltd.
// This file is part of provider <https://github.com/prosopo-io/provider>.
//
// provider is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// provider is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with provider.  If not, see <http://www.gnu.org/licenses/>.
#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
pub mod dapp {
    use prosopo::ProsopoRef;
    use ink_storage::{
        Mapping,
        traits::SpreadAllocate,
    };

    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Dapp {
        /// Total token supply.
        total_supply: Balance,
        /// Mapping from owner to number of owned token.
        balances: Mapping<AccountId, Balance>,
        /// Amount of tokens to drip feed via the faucet function
        faucet_amount: Balance,
        /// Token holder who initially receives all tokens
        token_holder: AccountId,
        /// The percentage of correct captchas that an Account must have answered correctly
        human_threshold: u8,
        /// The address of the prosopo bot protection contract
        prosopo_account: AccountId
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    /// Error types.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if not enough balance to fulfill a request is available.
        InsufficientBalance,
    }

    impl Dapp {
        /// Creates a new contract with the specified initial supply and loads an instance of the
        /// `prosopo` contract
        #[ink(constructor, payable)]
        pub fn new(initial_supply: Balance, faucet_amount: Balance, prosopo_account: AccountId, human_threshold: u8) -> Self {
            ink_lang::codegen::initialize_contract(|contract| Self::new_init(contract, initial_supply, faucet_amount, prosopo_account, human_threshold))
        }

        /// Default initializes the ERC-20 contract with the specified initial supply.
        fn new_init(&mut self, initial_supply: Balance, faucet_amount: Balance, prosopo_account: AccountId, human_threshold: u8) {
            let caller = Self::env().caller();
            self.balances.insert(&caller, &initial_supply);
            self.total_supply = initial_supply;
            self.faucet_amount = faucet_amount;
            self.token_holder = caller;
            self.human_threshold = human_threshold;
            self.prosopo_account = prosopo_account;
            // Events not working due to bug https://github.com/paritytech/ink/issues/1000
            // self.env().emit_event(Transfer {
            //     from: None,
            //     to: Some(caller),
            //     value: initial_supply,
            // });
        }

        /// Faucet function for sending tokens to humans
        #[ink(message)]
        pub fn faucet(&mut self, accountid: AccountId) {
            let token_holder = self.token_holder;
            if self.is_human(accountid, self.human_threshold) {
                self.transfer_from_to(&token_holder, &accountid, self.faucet_amount);
            }
        }

        /// Calls the `Prosopo` contract to check if `accountid` is human
        #[ink(message)]
        pub fn is_human(&self, accountid: AccountId, threshold: u8) -> bool {
            let mut prosopo_instance: ProsopoRef = ink_env::call::FromAccountId::from_account_id(self.prosopo_account);
            let last_correct_captcha = prosopo_instance.dapp_operator_last_correct_captcha(accountid).unwrap();
            // lets say that dapp requires confirmation every day
            let less_than_a_day_ago = last_correct_captcha.before_ms < 24 * 60 * 60 * 1000;
            prosopo_instance.dapp_operator_is_human_user(accountid, threshold).unwrap() && less_than_a_day_ago
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<(), Error> {
            let from = self.env().caller();
            self.transfer_from_to(&from, &to, value)
        }

        /// Transfers `value` amount of tokens from the caller's account to account `to`.
        ///
        /// On success a `Transfer` event is emitted.
        ///
        /// # Errors
        ///
        /// Returns `InsufficientBalance` error if there are not enough tokens on
        /// the caller's account balance.
        fn transfer_from_to(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<(), Error> {
            let from_balance = self.balance_of_impl(from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            self.balances.insert(from, &(from_balance - value));
            let to_balance = self.balance_of_impl(to);
            self.balances.insert(to, &(to_balance + value));
            // Events not working due to bug https://github.com/paritytech/ink/issues/1000
            // self.env().emit_event(Transfer {
            //     from: Some(*from),
            //     to: Some(*to),
            //     value,
            // });
            Ok(())
        }

        /// Returns the account balance for the specified `owner`.
        ///
        /// Returns `0` if the account is non-existent.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_impl(&owner)
        }

        /// Returns the account balance for the specified `owner`.
        ///
        /// Returns `0` if the account is non-existent.
        ///
        /// # Note
        ///
        /// Prefer to call this method over `balance_of` since this
        /// works using references which are more efficient in Wasm.
        #[inline]
        fn balance_of_impl(&self, owner: &AccountId) -> Balance {
            self.balances.get(owner).unwrap_or_default()
        }
    }

    #[cfg(test)]
    mod tests {
        use ink_env::hash::Blake2x256;
        use ink_env::hash::CryptoHash;
        use ink_env::hash::HashOutput;
        use ink_lang as ink;

        use super::*;

        use prosopo::Prosopo;
        use prosopo::prosopo::{ Payee, CaptchaStatus };

        /// Provider Register Helper
        fn generate_provider_data(id: u8, port: &str, fee: u32) -> (AccountId, Hash, u32) {
            let provider_account = AccountId::from([id; 32]);
            let service_origin = str_to_hash(format!("https://localhost:{}", port));

            (provider_account, service_origin, fee)
        }

        /// Helper function for converting string to Hash
        fn str_to_hash(str: String) -> Hash {
            let mut result = Hash::default();
            let len_result = result.as_ref().len();
            let mut hash_output = <<Blake2x256 as HashOutput>::Type as Default>::default();
            <Blake2x256 as CryptoHash>::hash((&str).as_ref(), &mut hash_output);
            let copy_len = core::cmp::min(hash_output.len(), len_result);
            result.as_mut()[0..copy_len].copy_from_slice(&hash_output[0..copy_len]);
            result
        }

        #[ink::test]
        fn test_is_human() {
            let contract = Dapp::new(1000, 1000, AccountId::from([0x1; 32]), 80);

            let operator_account = AccountId::from([0x2; 32]);

            // initialise the contract
            let mut prosopo_contract = Prosopo::default(operator_account);

            // Register the provider
            let (provider_account, service_origin, fee) = generate_provider_data(0x3, "4242", 0);
            prosopo_contract
                .provider_register(service_origin, fee, Payee::Provider, provider_account)
                .unwrap();

            // Call from the provider account to add data and stake tokens
            let balance = 100;
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(provider_account);
            let root = str_to_hash("merkle tree root".to_string());
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(balance);
            prosopo_contract.provider_update(service_origin, fee, Payee::Provider, provider_account);
            // can only add data set after staking
            prosopo_contract.provider_add_dataset(root).ok();

            // Register the dapp
            let dapp_caller_account = AccountId::from([0x4; 32]);
            let dapp_contract_account = AccountId::from([0x5; 32]);

            // Call from the dapp account
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(dapp_caller_account);
            // Give the dap a balance
            let balance = 100;
            ink_env::test::set_value_transferred::<ink_env::DefaultEnvironment>(balance);
            let client_origin = service_origin.clone();
            prosopo_contract.dapp_register(client_origin, dapp_contract_account, None);

            //Dapp User commit
            let dapp_user_account = AccountId::from([0x6; 32]);
            // Call from the Dapp User Account
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(dapp_user_account);
            let user_root = str_to_hash("user merkle tree root".to_string());
            prosopo_contract
                .dapp_user_commit(dapp_contract_account, root, user_root, provider_account)
                .ok();

            // Call from the provider account to mark the solution as approved
            ink_env::test::set_caller::<ink_env::DefaultEnvironment>(provider_account);
            let solution_id = user_root;
            prosopo_contract.provider_approve(solution_id, 100);

            // not sure what to do assert here since in test env, blockstamps are always 0
            // TODO (thread 'dapp::tests::test_is_human' panicked at 'not implemented: off-chain environment does not support contract invocation):
            // assert_eq!(contract.is_human(dapp_user_account, contract.human_threshold), false);
        }
    }
}
