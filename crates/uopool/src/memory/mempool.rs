use crate::mempool::Mempool;
use educe::Educe;
use ethers::types::{Address, U256};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::collections::{HashMap, HashSet};

#[derive(Default, Educe)]
#[educe(Debug)]
pub struct MemoryMempool {
    /// A [HashMap] of [UserOperationHash](UserOperationHash) to [UserOperation](UserOperation) to
    /// look up by hash
    user_operations: HashMap<UserOperationHash, UserOperation>, // user_operation_hash -> user_operation
    /// A [Hashmap](std::collections::HashMap) of [Address] to [HashSet] of
    /// [UserOperationHash](UserOperationHash) for lookups by sender
    user_operations_by_sender: HashMap<Address, HashSet<UserOperationHash>>, // sender -> user_operations
    /// A [Hashmap](std::collections::HashMap) of [UserOperationHash](UserOperationHash) to [Vec] of
    /// [CodeHash](CodeHash) for lookups by [UserOperationHash](UserOperationHash)
    code_hashes_by_user_operation: HashMap<UserOperationHash, Vec<CodeHash>>, // user_operation_hash -> (contract_address -> code_hash)
    /// A [Hashmap](std::collections::HashMap) of [Address] to [HashSet] of
    /// [UserOperationHash](UserOperationHash) for lookups by entity
    user_operations_by_entity: HashMap<Address, HashSet<UserOperationHash>>, // entity -> user_operations
}

impl Mempool for MemoryMempool {
    type Error = eyre::Error;

    /// Adds a [UserOperation](UserOperation) to the mempool
    ///
    /// # Arguments
    /// * `uo` - The [UserOperation](UserOperation) to add
    /// * `ep` - The [Address](Address) of the endpoint
    /// * `chain_id` - The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) Chain ID.
    ///
    /// # Returns
    /// * `Ok(UserOperationHash)` - The hash of the [UserOperation](UserOperation) that was added
    /// * `Err(eyre::Error)` - If the [UserOperation](UserOperation) could not be added
    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> eyre::Result<UserOperationHash> {
        let uo_hash = uo.hash(ep, chain_id);
        let (sender, factory, paymaster) = uo.get_entities();

        self.user_operations_by_sender
            .entry(sender)
            .or_default()
            .insert(uo_hash);
        if let Some(factory) = factory {
            self.user_operations_by_entity
                .entry(factory)
                .or_default()
                .insert(uo_hash);
        }
        if let Some(paymaster) = paymaster {
            self.user_operations_by_entity
                .entry(paymaster)
                .or_default()
                .insert(uo_hash);
        }
        self.user_operations.insert(uo_hash, uo);

        Ok(uo_hash)
    }

    /// Gets a [UserOperation](UserOperation) by its hash
    ///
    /// # Arguments
    /// * `uo_hash` - The hash of the [UserOperation](UserOperation) to get
    ///
    /// # Returns
    /// * `Ok(Some(UserOperation))` - The [UserOperation](UserOperation) if it exists
    /// * `Ok(None)` - If the [UserOperation](UserOperation) does not exist
    /// * `Err(eyre::Error)` - If the [UserOperation](UserOperation) could not be retrieved
    fn get(&self, uo_hash: &UserOperationHash) -> eyre::Result<Option<UserOperation>> {
        Ok(self.user_operations.get(uo_hash).cloned())
    }

    /// Gets all [UserOperation](UserOperation)s by sender
    ///
    /// # Arguments
    /// * `addr` - The [Address](Address) of the sender
    ///
    /// # Returns
    /// * `Vec<UserOperation>` - An array of [UserOperations](UserOperation) if they exist.
    /// Otherwise, an empty array.
    fn get_all_by_sender(&self, addr: &Address) -> Vec<UserOperation> {
        return if let Some(uos_by_sender) = self.user_operations_by_sender.get(addr) {
            uos_by_sender
                .iter()
                .filter_map(|uo_hash| self.user_operations.get(uo_hash).cloned())
                .collect()
        } else {
            vec![]
        };
    }

    /// Gets the number of [UserOperation](UserOperation)s by sender
    ///
    /// # Arguments
    /// * `addr` - The [Address](Address) of the sender
    ///
    /// # Returns
    /// * `usize` - The number of [UserOperations](UserOperation) if they exist. Otherwise, 0.
    fn get_number_by_sender(&self, addr: &Address) -> usize {
        return if let Some(uos_by_sender) = self.user_operations_by_sender.get(addr) {
            uos_by_sender.len()
        } else {
            0
        };
    }

    /// Gets the number of [UserOperation](UserOperation)s by entity
    ///
    /// # Arguments
    /// * `addr` - The [Address](Address) of the sender
    ///
    /// # Returns
    /// * `usize` - The number of [UserOperations](UserOperation) if they exist. Otherwise, 0.
    fn get_number_by_entity(&self, addr: &Address) -> usize {
        return if let Some(uos_by_entity) = self.user_operations_by_entity.get(addr) {
            uos_by_entity.len()
        } else {
            0
        };
    }

    /// Gets [CodeHash](CodeHash) by [UserOperationHash](UserOperationHash)
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) of the [UserOperation](UserOperation)
    ///
    /// # Returns
    /// * `Ok(bool)` - True if the [CodeHash](CodeHash) exists. Otherwise, false.
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> eyre::Result<bool> {
        Ok(self.code_hashes_by_user_operation.contains_key(uo_hash))
    }

    /// Sets [CodeHash](CodeHash) by [UserOperationHash](UserOperationHash)
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) of the [UserOperation](UserOperation)
    /// * `hashes` - The [CodeHash](CodeHash) to set
    ///
    /// # Returns
    /// * `Ok(())` - If the [CodeHash](CodeHash) was set
    /// * `Err(eyre::Error)` - If the [CodeHash](CodeHash) could not be set
    fn set_code_hashes(
        &mut self,
        uo_hash: &UserOperationHash,
        hashes: &Vec<CodeHash>,
    ) -> eyre::Result<(), Self::Error> {
        self.code_hashes_by_user_operation
            .insert(*uo_hash, hashes.clone());
        Ok(())
    }

    /// Gets [CodeHash](CodeHash) by [UserOperationHash](UserOperationHash)
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) of the [UserOperation](UserOperation)
    ///
    /// # Returns
    /// * `Vec<CodeHash>` - An array of [CodeHash](CodeHash) if they exist. Otherwise, an empty array.
    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Vec<CodeHash> {
        if let Some(hashes) = self.code_hashes_by_user_operation.get(uo_hash) {
            hashes.clone()
        } else {
            vec![]
        }
    }

    /// Removes a [UserOperation](UserOperation) by its hash
    ///
    /// # Arguments
    /// * `uo_hash` - The hash of the [UserOperation](UserOperation) to remove
    ///
    /// # Returns
    /// * `Ok(())` - If the [UserOperation](UserOperation) was removed
    /// * `Err(eyre::Error)` - If the [UserOperation](UserOperation) could not be removed
    fn remove(&mut self, uo_hash: &UserOperationHash) -> eyre::Result<()> {
        let uo: UserOperation;

        if let Some(user_op) = self.user_operations.get(uo_hash) {
            uo = user_op.clone();
        } else {
            return Err(eyre::eyre!("User operation not found"));
        }

        let (sender, factory, paymaster) = uo.get_entities();

        self.user_operations.remove(uo_hash);

        if let Some(uos) = self.user_operations_by_sender.get_mut(&sender) {
            uos.remove(uo_hash);

            if uos.is_empty() {
                self.user_operations_by_sender.remove(&sender);
            }
        }

        if let Some(factory) = factory {
            if let Some(uos) = self.user_operations_by_entity.get_mut(&factory) {
                uos.remove(uo_hash);

                if uos.is_empty() {
                    self.user_operations_by_entity.remove(&factory);
                }
            }
        }

        if let Some(paymaster) = paymaster {
            if let Some(uos) = self.user_operations_by_entity.get_mut(&paymaster) {
                uos.remove(uo_hash);

                if uos.is_empty() {
                    self.user_operations_by_entity.remove(&paymaster);
                }
            }
        }

        self.code_hashes_by_user_operation.remove(uo_hash);

        Ok(())
    }

    /// Removes a [UserOperation](UserOperation) by its entity
    ///
    /// # Arguments
    /// * `entity` - The [Address](Address) of the entity
    ///
    /// # Returns
    /// * `Ok(())` - If the [UserOperation](UserOperation) was removed
    /// * `Err(eyre::Error)` - If the [UserOperation](UserOperation) could not be removed
    fn remove_by_entity(&mut self, entity: &Address) -> eyre::Result<()> {
        let uos = self.user_operations_by_entity.get(entity).cloned();

        if let Some(uos) = uos {
            for uo_hash in uos {
                self.remove(&uo_hash)?;
            }
        }

        Ok(())
    }

    /// Sorts the [UserOperations](UserOperation) by `max_priority_fee_per_gas` and `nonce`
    ///
    /// # Returns
    /// * `Ok(Vec<UserOperation>)` - The sorted [UserOperations](UserOperation)
    fn get_sorted(&self) -> eyre::Result<Vec<UserOperation>> {
        let mut uos: Vec<UserOperation> = self.user_operations.values().cloned().collect();
        uos.sort_by(|a, b| {
            if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
            } else {
                a.nonce.cmp(&b.nonce)
            }
        });
        Ok(uos)
    }

    /// Gets all [UserOperations](UserOperation)
    ///
    /// # Returns
    /// * `Vec<UserOperation>` - All [UserOperations](UserOperation)
    fn get_all(&self) -> Vec<UserOperation> {
        self.user_operations.values().cloned().collect()
    }

    /// Clears the [UserOperations](UserOperation) from the mempool
    ///
    /// # Returns
    /// None
    fn clear(&mut self) {
        self.user_operations.clear();
        self.user_operations_by_sender.clear();
        self.code_hashes_by_user_operation.clear();
        self.user_operations_by_entity.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tests::mempool_test_case;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn memory_mempool() {
        let mempool = MemoryMempool::default();
        mempool_test_case(mempool, "User operation not found");
    }
}
