#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod content_ownership {
    use ink::storage::Mapping;
    use ink::prelude::string::String;
    use ink::prelude::collections::BTreeMap;

    /// A simple structure representing a digital content record.
    /// It holds an IPFS (or similar) content hash and the current owner's AccountId.
    #[derive(scale::Encode, scale::Decode, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Content {
        content_hash: String,
        owner: AccountId,
    }

    /// Custom error type for the contract.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Returned if a function is called by someone other than the admin.
        NotAdmin = 0,
        /// Returned if content with the given id is not found.
        ContentNotFound = 1,
        /// Returned if a function is called by someone other than the content owner.
        NotOwner = 2,
        /// Returned if the counter overflow.
        CounterOverflow = 3,
    }

    /// A type alias for the contract's `Result` type.
    pub type Result<T> = core::result::Result<T, Error>;

    /// The ContentOwnership contract maintains on‑chain tracking of digital content and its owner.
    #[ink(storage)]
    pub struct ContentOwnership {
        /// The administrator of the contract.
        admin: AccountId,
        /// Oracle data which serves as a trusted reference. It could be, for example, a hash of licensing terms.
        oracle_data: String,
        /// Mapping of content IDs to content records.
        contents: Mapping<u64, Content>,
        /// A counter for generating unique content IDs.
        next_content_id: u64,
        /// Mapping of content hashes to content IDs.
        content_hash_to_id: BTreeMap<String, u64>,
    }

    impl ContentOwnership {
        /// Constructor: upon deployment, the deployer becomes the admin and provides an initial piece of oracle data.
        ///
        /// In a production scenario, this oracle data might be a hash of off‑chain licensing details or the current policy version.
        #[ink(constructor)]
        pub fn new(initial_oracle_data: String) -> Self {
            Self {
                admin: Self::env().caller(),
                oracle_data: initial_oracle_data,
                contents: Mapping::default(),
                next_content_id: 1,
                content_hash_to_id: BTreeMap::new(),
            }
        }

        /// Update the stored oracle data.
        /// Only the admin (contract deployer) is allowed to update this field.
        #[ink(message)]
        pub fn update_oracle_data(&mut self, new_data: String) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::NotAdmin);
            }
            self.oracle_data = new_data;
            Ok(())
        }

        /// Register new digital content on-chain.
        ///
        /// The caller submits the content hash (e.g. an IPFS hash) and the contract stores it together with the caller’s AccountId.
        /// Returns a unique content identifier.
        #[ink(message)]
        pub fn register_content(&mut self, content_hash: String) -> Result<u64> {
            if self.content_hash_to_id.contains_key(&content_hash) {
                return Ok(*self.content_hash_to_id.get(&content_hash).unwrap());
            }
            let caller = self.env().caller();
            let content_id = self.next_content_id;
            self.next_content_id = self.next_content_id.checked_add(1).ok_or(Error::CounterOverflow)?;
            let record = Content {
                content_hash: content_hash.clone(),
                owner: caller,
            };
            self.contents.insert(content_id, &record);
            self.content_hash_to_id.insert(content_hash, content_id);
            Ok(content_id)
        }

        /// Transfer ownership of a registered content item.
        ///
        /// The caller must be the current owner in order to authorize the transfer.
        #[ink(message)]
        pub fn transfer_ownership(
            &mut self,
            content_id: u64,
            new_owner: AccountId,
        ) -> Result<()> {
            let mut record = self.contents.get(content_id).ok_or(Error::ContentNotFound)?;
            if self.env().caller() != record.owner {
                return Err(Error::NotOwner);
            }
            record.owner = new_owner;
            self.contents.insert(content_id, &record);
            Ok(())
        }

        /// Retrieve the content record by its unique identifier.
        #[ink(message)]
        pub fn get_content(&self, content_id: u64) -> Option<Content> {
            self.contents.get(content_id)
        }

        /// Return the current oracle data stored in the contract.
        #[ink(message)]
        pub fn get_oracle_data(&self) -> String {
            self.oracle_data.clone()
        }
    }
}
