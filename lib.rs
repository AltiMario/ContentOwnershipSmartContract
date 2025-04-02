#![cfg_attr(not(feature = "std"), no_std, no_main)]

/// The `content_ownership` module defines a smart contract for managing digital content ownership.
/// It allows users to register digital content, transfer ownership, and validate content using oracle data.
#[ink::contract]
mod content_ownership {
    use ink::storage::Mapping;
    use ink::prelude::string::String;
    use ink::prelude::collections::BTreeMap;

    /// Represents a digital content record stored on-chain.
    /// Each record contains:
    /// - `content_hash`: A unique identifier for the content (e.g., an IPFS hash).
    /// - `owner`: The AccountId of the current owner of the content.
    #[derive(scale::Encode, scale::Decode, Clone, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
    pub struct Content {
        content_hash: String,
        owner: AccountId,
    }

    /// Defines custom error types for the contract.
    /// These errors are returned when specific conditions are not met.
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Error returned when a non-admin user attempts an admin-only action.
        NotAdmin = 0,
        /// Error returned when a content ID is not found in the storage.
        ContentNotFound = 1,
        /// Error returned when a non-owner attempts to transfer ownership.
        NotOwner = 2,
        /// Error returned when the content ID counter overflows.
        CounterOverflow = 3,
        /// Error returned when the content hash is deemed invalid by the oracle.
        InvalidContent = 4,
    }

    /// A type alias for the contract's result type.
    /// It wraps the `Result` type with the contract's custom `Error` enum.
    pub type Result<T> = core::result::Result<T, Error>;

    /// The `ContentOwnership` contract manages digital content and its ownership.
    /// It provides functionality for:
    /// - Registering new content.
    /// - Transferring ownership of content.
    /// - Validating content using oracle data.
    #[ink(storage)]
    pub struct ContentOwnership {
        /// The administrator of the contract, typically the deployer.
        admin: AccountId,
        /// Oracle data used for validating content (e.g., a hash of licensing terms).
        oracle_data: String,
        /// A mapping of content IDs to their corresponding content records.
        contents: Mapping<u64, Content>,
        /// A counter for generating unique content IDs.
        next_content_id: u64,
        /// A mapping of content hashes to their corresponding content IDs.
        content_hash_to_id: BTreeMap<String, u64>,
    }

    //----------------------------------
    // Default Implementation
    //----------------------------------

    /// Provides default initialization values for the contract.
    /// This is primarily used for testing or demonstration purposes.
    impl Default for ContentOwnership {
        fn default() -> Self {
            Self {
                admin: AccountId::from([0u8; 32]),
                oracle_data: String::from("default_oracle"),
                contents: Mapping::default(),
                next_content_id: 1,
                content_hash_to_id: BTreeMap::new(),
            }
        }
    }

    //----------------------------------
    // Contract Implementation
    //----------------------------------

    impl ContentOwnership {
        /// Constructor: Initializes the contract with the deployer as the admin and sets the initial oracle data.
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                admin: Self::env().caller(),
                ..Default::default()
            }
        }

        /// Updates the oracle data stored in the contract.
        /// Only the admin can call this function.
        ///
        /// # Arguments
        /// - `new_data`: The new oracle data to be stored.
        ///
        /// # Errors
        /// - Returns `Error::NotAdmin` if the caller is not the admin.
        #[ink(message)]
        pub fn update_oracle_data(&mut self, new_data: String) -> Result<()> {
            if self.env().caller() != self.admin {
                return Err(Error::NotAdmin);
            }
            self.oracle_data = new_data;
            Ok(())
        }

        /// Registers new digital content on-chain.
        /// The caller provides a content hash, which is validated against the oracle data.
        /// If valid, the content is stored with the caller as the owner.
        ///
        /// # Arguments
        /// - `content_hash`: The unique hash representing the content (e.g., an IPFS hash).
        ///
        /// # Returns
        /// - A unique content ID for the registered content.
        ///
        /// # Errors
        /// - Returns `Error::InvalidContent` if the content hash is invalid.
        /// - Returns `Error::CounterOverflow` if the content ID counter overflows.
        #[ink(message)]
        pub fn register_content(&mut self, content_hash: String) -> Result<u64> {
            if !self.validate_content_with_oracle(&content_hash) {
                return Err(Error::InvalidContent);
            }

            if self.content_hash_to_id.contains_key(&content_hash) {
                return Ok(*self.content_hash_to_id.get(&content_hash).unwrap());
            }
            let caller = self.env().caller();
            let content_id = self.next_content_id;
            self.next_content_id = self.next_content_id
                .checked_add(1)
                .ok_or(Error::CounterOverflow)?;
            let record = Content {
                content_hash: content_hash.clone(),
                owner: caller,
            };
            self.contents.insert(content_id, &record);
            self.content_hash_to_id.insert(content_hash, content_id);
            Ok(content_id)
        }

        /// Validates a content hash against the oracle data.
        ///
        /// # Arguments
        /// - `content_hash`: The hash to validate.
        ///
        /// # Returns
        /// - `true` if the content hash is valid, `false` otherwise.
        fn validate_content_with_oracle(&self, content_hash: &str) -> bool {
            content_hash.starts_with(&self.oracle_data)
        }

        /// Transfers ownership of a registered content item to a new owner.
        /// Only the current owner can authorize the transfer.
        ///
        /// # Arguments
        /// - `content_id`: The unique ID of the content to transfer.
        /// - `new_owner`: The AccountId of the new owner.
        ///
        /// # Errors
        /// - Returns `Error::ContentNotFound` if the content ID is not found.
        /// - Returns `Error::NotOwner` if the caller is not the current owner.
        #[ink(message)]
        pub fn transfer_ownership(&mut self, content_id: u64, new_owner: AccountId) -> Result<()> {
            let mut record = self.contents.get(content_id).ok_or(Error::ContentNotFound)?;
            if self.env().caller() != record.owner {
                return Err(Error::NotOwner);
            }
            record.owner = new_owner;
            self.contents.insert(content_id, &record);
            Ok(())
        }

        /// Retrieves a content record by its unique identifier.
        ///
        /// # Arguments
        /// - `content_id`: The unique ID of the content to retrieve.
        ///
        /// # Returns
        /// - An `Option` containing the content record if found, or `None` if not found.
        #[ink(message)]
        pub fn get_content(&self, content_id: u64) -> Option<Content> {
            self.contents.get(content_id)
        }

        /// Returns the current oracle data stored in the contract.
        ///
        /// # Returns
        /// - A `String` containing the oracle data.
        #[ink(message)]
        pub fn get_oracle_data(&self) -> String {
            self.oracle_data.clone()
        }
    }
}
