use super::env::Env;
use super::{
    env::DBError,
    tables::{CodeHashes, UserOperations, UserOperationsByEntity, UserOperationsBySender},
    utils::{WrapAddress, WrapUserOperation, WrapUserOperationHash},
};
use crate::mempool::Mempool;
use ethers::types::{Address, U256};
use reth_db::cursor::DbDupCursorRO;
use reth_db::{
    cursor::DbCursorRO,
    database::Database,
    mdbx::EnvironmentKind,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::{simulation::CodeHash, UserOperation, UserOperationHash};
use std::sync::Arc;

/// The database-based implementation of the [Mempool](crate::mempool::Mempool) trait.
#[derive(Debug)]
pub struct DatabaseMempool<E: EnvironmentKind> {
    env: Arc<Env<E>>,
}

impl<E: EnvironmentKind> DatabaseMempool<E> {
    pub fn new(env: Arc<Env<E>>) -> Self {
        Self { env }
    }
}

impl<E: EnvironmentKind> Mempool for DatabaseMempool<E> {
    type Error = DBError;

    /// Adds a [UserOperation](UserOperation) to the mempool database.
    ///
    /// # Arguments
    /// * `uo` - The user operation to add.
    /// * `ep` - The entry point address.
    /// * `chain_id` - The [EIP-155](https://eips.ethereum.org/EIPS/eip-155) Chain ID.
    ///
    /// # Returns
    /// * `Ok(UserOperationHash)` - The hash of the user operation.
    /// * `Err(DBError)` - The database error.
    fn add(
        &mut self,
        uo: UserOperation,
        ep: &Address,
        chain_id: &U256,
    ) -> Result<UserOperationHash, DBError> {
        let hash = uo.hash(ep, chain_id);
        let tx = self.env.tx_mut()?;

        let uo_hash_wrap: WrapUserOperationHash = hash.into();
        let uo_wrap: WrapUserOperation = uo.clone().into();
        let (sender, factory, paymaster) = uo.get_entities();

        tx.put::<UserOperations>(uo_hash_wrap.clone(), uo_wrap.clone())?;
        tx.put::<UserOperationsBySender>(sender.into(), uo_hash_wrap.clone())?;
        if let Some(factory) = factory {
            tx.put::<UserOperationsByEntity>(factory.into(), uo_hash_wrap.clone())?;
        }
        if let Some(paymaster) = paymaster {
            tx.put::<UserOperationsByEntity>(paymaster.into(), uo_hash_wrap)?;
        }

        tx.commit()?;
        Ok(hash)
    }

    /// Gets a [UserOperation](UserOperation) given its [hash](UserOperationHash) from the mempool database
    ///
    /// # Arguments
    /// * `uo_hash` - The [hash of the user operation](UserOperationHash).
    ///
    /// # Returns
    /// * `Ok(Option<UserOperation>)` - The user operation if it exists.
    /// * `Err(DBError)` - The database error.
    fn get(&self, uo_hash: &UserOperationHash) -> Result<Option<UserOperation>, DBError> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<UserOperations>(uo_hash_wrap)?;
        tx.commit()?;

        Ok(res.map(|uo| uo.into()))
    }

    /// Get all [UserOperations](UserOperation) from the mempool database given a sender [Address](Address).
    ///
    /// # Arguments
    /// * `sender` - The sender [Address](Address).
    ///
    /// # Returns
    /// * `Vec<UserOperation>` - An array of [UserOperations](UserOperation) from the given sender.
    fn get_all_by_sender(&self, sender: &Address) -> Vec<UserOperation> {
        let sender_wrap: WrapAddress = (*sender).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<UserOperationsBySender>()?;
                // https://github.com/ralexstokes/reth/blob/ebd5d3c1a2645119330f1dbdd759c995c4f0947c/crates/stages/src/trie/mod.rs#L242
                let mut curr =
                    cursor.seek_by_key_subkey(sender_wrap.clone(), Address::default().into())?;

                let mut v: Vec<WrapUserOperationHash> = vec![];
                while let Some(uo_hash) = curr {
                    v.push(uo_hash);
                    curr = cursor.next_dup()?.map(|(_, v)| v);
                }

                let res: Vec<UserOperation> = v
                    .iter()
                    .filter_map(|uo_hash| tx.get::<UserOperations>(uo_hash.clone()).ok())
                    .filter_map(|uo_wrap| uo_wrap.map(|uo| uo.into()))
                    .collect();
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    /// Gets the number of [UserOperations](UserOperation) from the mempool database given a sender [Address].
    ///
    /// # Arguments
    /// * `addr` - The sender [Address](Address).
    ///
    /// # Returns
    /// * `usize` - The number of [UserOperations](UserOperation) from the given sender.
    fn get_number_by_sender(&self, addr: &Address) -> usize {
        let addr_wrap: WrapAddress = (*addr).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<UserOperationsBySender>()?;
                let mut curr =
                    cursor.seek_by_key_subkey(addr_wrap.clone(), Address::default().into())?;

                let mut c: usize = 0;
                while curr.is_some() {
                    c += 1;
                    curr = cursor.next_dup()?.map(|(_, v)| v);
                }

                tx.commit()?;
                Ok(c)
            })
            .unwrap_or(0)
    }

    /// Gets the number of [UserOperations](UserOperation) from the mempool database given a entity [Address].
    ///
    /// # Arguments
    /// * `addr` - The entity [Address](Address).
    ///
    /// # Returns
    /// * `usize` - The number of [UserOperations](UserOperation) from the given entity.
    fn get_number_by_entity(&self, addr: &Address) -> usize {
        let addr_wrap: WrapAddress = (*addr).into();
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<UserOperationsByEntity>()?;
                let mut curr =
                    cursor.seek_by_key_subkey(addr_wrap.clone(), Address::default().into())?;

                let mut c: usize = 0;
                while curr.is_some() {
                    c += 1;
                    curr = cursor.next_dup()?.map(|(_, v)| v);
                }

                tx.commit()?;
                Ok(c)
            })
            .unwrap_or(0)
    }

    /// Gets the number of [UserOperation](UserOperation)s by sender from the mempool database.
    ///
    /// # Arguments
    /// * `addr` - The [Address](Address) of the sender
    ///
    /// # Returns
    /// * `usize` - The number of [UserOperations](UserOperation) if they exist. Otherwise, 0.
    fn has_code_hashes(&self, uo_hash: &UserOperationHash) -> Result<bool, Self::Error> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap)?;
        tx.commit()?;
        Ok(res.is_some())
    }

    /// Gets [CodeHash](CodeHash) by [UserOperationHash](UserOperationHash) from the mempool database
    ///
    /// # Arguments
    /// * `uo_hash` - The [UserOperationHash](UserOperationHash) of the [UserOperation](UserOperation)
    ///
    /// # Returns
    /// * `Ok(bool)` - True if the [CodeHash](CodeHash) exists. Otherwise, false.
    fn get_code_hashes(&self, uo_hash: &UserOperationHash) -> Vec<CodeHash> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_dup_read::<CodeHashes>()?;
                let mut curr =
                    cursor.seek_by_key_subkey(uo_hash_wrap.clone(), Address::default().into())?;

                let mut v: Vec<CodeHash> = vec![];
                while let Some(ch) = curr {
                    v.push(ch.into());
                    curr = cursor.next_dup()?.map(|(_, v)| v);
                }

                tx.commit()?;
                Ok(v)
            })
            .unwrap_or_else(|_| vec![])
    }

    /// Sets [CodeHash](CodeHash) by [UserOperationHash](UserOperationHash) in the mempool database
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
    ) -> Result<(), Self::Error> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        let res = tx.get::<CodeHashes>(uo_hash_wrap.clone())?;
        if res.is_some() {
            tx.delete::<CodeHashes>(uo_hash_wrap.clone(), None)?;
        }
        for hash in hashes {
            tx.put::<CodeHashes>(uo_hash_wrap.clone(), hash.clone().into())?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Removes a [UserOperation](UserOperation) by its hash from the mempool database
    ///
    /// # Arguments
    /// * `uo_hash` - The hash of the [UserOperation](UserOperation) to remove
    ///
    /// # Returns
    /// * `Ok(())` - If the [UserOperation](UserOperation) was removed
    /// * `Err(eyre::Error)` - If the [UserOperation](UserOperation) could not be removed
    fn remove(&mut self, uo_hash: &UserOperationHash) -> Result<(), DBError> {
        let uo_hash_wrap: WrapUserOperationHash = (*uo_hash).into();

        let tx = self.env.tx_mut()?;
        if let Some(uo_wrap) = tx.get::<UserOperations>(uo_hash_wrap.clone())? {
            let uo: UserOperation = uo_wrap.into();
            let (sender, factory, paymaster) = uo.get_entities();

            tx.delete::<UserOperations>(uo_hash_wrap.clone(), None)?;
            tx.delete::<UserOperationsBySender>(sender.into(), Some(uo_hash_wrap.clone()))?;
            tx.delete::<CodeHashes>(uo_hash_wrap.clone(), None)?;

            if let Some(factory) = factory {
                tx.delete::<UserOperationsByEntity>(factory.into(), Some(uo_hash_wrap.clone()))?;
            }
            if let Some(paymaster) = paymaster {
                tx.delete::<UserOperationsByEntity>(paymaster.into(), Some(uo_hash_wrap))?;
            }

            tx.commit()?;
            Ok(())
        } else {
            Err(DBError::NotFound)
        }
    }

    /// Removes all [UserOperations](UserOperation) by entity
    ///
    /// # Arguments
    /// * `entity` - The [Address](Address) of the entity
    ///
    /// # Returns
    /// * `Ok(())` - If the [UserOperations](UserOperation) were removed
    /// * `Err(eyre::Error)` - If the [UserOperations](UserOperation) could not be removed
    fn remove_by_entity(&mut self, entity: &Address) -> Result<(), Self::Error> {
        let entity_wrap: WrapAddress = (*entity).into();

        let tx = self.env.tx()?;
        let mut cursor = tx.cursor_dup_read::<UserOperationsByEntity>()?;
        let mut curr = cursor.seek_by_key_subkey(entity_wrap.clone(), Address::default().into())?;

        let mut v: Vec<WrapUserOperationHash> = vec![];
        while let Some(uo_hash) = curr {
            v.push(uo_hash);
            curr = cursor.next_dup()?.map(|(_, v)| v);
        }

        tx.commit()?;

        for uo_hash_wrap in v {
            self.remove(&uo_hash_wrap.into())?;
        }

        Ok(())
    }

    /// Sorts the [UserOperations](UserOperation) by `max_priority_fee_per_gas` and `nonce`
    ///
    /// # Returns
    /// * `Ok(Vec<UserOperation>)` - The sorted [UserOperations](UserOperation)
    fn get_sorted(&self) -> Result<Vec<UserOperation>, DBError> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut cursor = tx.cursor_read::<UserOperations>()?;
                let mut uos: Vec<UserOperation> = cursor
                    .walk(Some(WrapUserOperationHash::default()))?
                    .map(|a| a.map(|(_, uo)| uo.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                uos.sort_by(|a, b| {
                    if a.max_priority_fee_per_gas != b.max_priority_fee_per_gas {
                        b.max_priority_fee_per_gas.cmp(&a.max_priority_fee_per_gas)
                    } else {
                        a.nonce.cmp(&b.nonce)
                    }
                });
                Ok(uos)
            })
            .map_err(DBError::DBInternalError)
    }

    /// Gets all [UserOperations](UserOperation) from the mempool database
    ///
    /// # Returns
    /// * `Vec<UserOperation>` - All [UserOperations](UserOperation)
    fn get_all(&self) -> Vec<UserOperation> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<UserOperations>()?;
                let res: Vec<UserOperation> = c
                    .walk(Some(WrapUserOperationHash::default()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }

    /// Clears the [UserOperations](UserOperation) from the mempool database
    ///
    /// # Returns
    /// None
    fn clear(&mut self) {
        self.env
            .tx_mut()
            .and_then(|tx| {
                tx.clear::<UserOperations>()?;
                tx.clear::<UserOperationsBySender>()?;
                tx.clear::<UserOperationsByEntity>()?;
                tx.commit()
            })
            .expect("Clear database failed");
    }
}

#[cfg(test)]
mod tests {
    use crate::{database::init_env, utils::tests::mempool_test_case, DatabaseMempool};
    use reth_libmdbx::WriteMap;
    use std::sync::Arc;
    use tempdir::TempDir;

    #[allow(clippy::unit_cmp)]
    #[tokio::test]
    async fn database_mempool() {
        let dir = TempDir::new("test-silius-db").unwrap();

        let env = init_env::<WriteMap>(dir.into_path()).unwrap();
        env.create_tables()
            .expect("Create mdbx database tables failed");
        let mempool: DatabaseMempool<WriteMap> = DatabaseMempool::new(Arc::new(env));

        mempool_test_case(mempool, "NotFound");
    }
}
