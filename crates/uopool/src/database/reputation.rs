use super::{tables::EntitiesReputation, utils::WrapAddress, DatabaseTable};
use crate::{
    mempool::ClearOp,
    reputation::{ReputationEntryOp, ReputationOpError},
};
use ethers::types::Address;
use reth_db::{
    cursor::{DbCursorRO, DbCursorRW},
    database::Database,
    mdbx::EnvironmentKind,
    transaction::{DbTx, DbTxMut},
};
use silius_primitives::reputation::ReputationEntry;

impl<E: EnvironmentKind> ClearOp for DatabaseTable<E, EntitiesReputation> {
    fn clear(&mut self) {
        let tx = self.env.tx_mut().expect("clear database tx should work");
        tx.clear::<EntitiesReputation>().expect("clear succeed");
        tx.commit().expect("clear commit succeed");
    }
}

impl<E: EnvironmentKind> ReputationEntryOp for DatabaseTable<E, EntitiesReputation> {
    fn get_entry(&self, addr: &Address) -> Result<Option<ReputationEntry>, ReputationOpError> {
        let addr_wrap: WrapAddress = (*addr).into();

        let tx = self.env.tx()?;
        let res = tx.get::<EntitiesReputation>(addr_wrap)?;
        tx.commit()?;
        Ok(res.map(|o| o.into()))
    }

    fn set_entry(
        &mut self,
        addr: &Address,
        entry: ReputationEntry,
    ) -> Result<Option<ReputationEntry>, ReputationOpError> {
        let tx = self.env.tx_mut()?;
        let original = tx.get::<EntitiesReputation>((*addr).into())?;
        tx.put::<EntitiesReputation>((*addr).into(), entry.into())?;
        tx.commit()?;
        Ok(original.map(|o| o.into()))
    }

    fn contains_entry(&self, addr: &Address) -> Result<bool, ReputationOpError> {
        Ok(self.get_entry(addr)?.is_some())
    }

    fn update(&mut self) -> Result<(), ReputationOpError> {
        let tx = self.env.tx_mut()?;
        let mut cursor = tx.cursor_write::<EntitiesReputation>()?;

        while let Ok(Some((addr_wrap, ent))) = cursor.next() {
            let mut ent: ReputationEntry = ent.into();
            ent.uo_seen = ent.uo_seen * 23 / 24;
            ent.uo_included = ent.uo_included * 23 / 24;

            if ent.uo_seen > 0 || ent.uo_included > 0 {
                cursor.upsert(addr_wrap, ent.into())?;
            } else {
                cursor.delete_current()?;
            }
        }

        tx.commit()?;

        Ok(())
    }

    fn get_all(&self) -> Vec<ReputationEntry> {
        self.env
            .tx()
            .and_then(|tx| {
                let mut c = tx.cursor_read::<EntitiesReputation>()?;
                let res: Vec<ReputationEntry> = c
                    .walk(Some(WrapAddress::default()))?
                    .map(|a| a.map(|(_, v)| v.into()))
                    .collect::<Result<Vec<_>, _>>()?;
                tx.commit()?;
                Ok(res)
            })
            .unwrap_or_else(|_| vec![])
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        database::{init_env, tables::EntitiesReputation, DatabaseTable},
        utils::tests::reputation_test_case,
        Reputation,
    };
    use ethers::types::{Address, U256};
    use reth_libmdbx::WriteMap;
    use silius_primitives::consts::reputation::{
        BAN_SLACK, MIN_INCLUSION_RATE_DENOMINATOR, THROTTLING_SLACK,
    };
    use std::{collections::HashSet, sync::Arc};
    use tempdir::TempDir;

    #[tokio::test]
    async fn database_reputation() {
        let dir = TempDir::new("test-silius-db").unwrap();

        let env = init_env::<WriteMap>(dir.into_path()).unwrap();
        env.create_tables()
            .expect("Create mdbx database tables failed");
        let env = Arc::new(env);
        let entry: DatabaseTable<WriteMap, EntitiesReputation> = DatabaseTable::new(env.clone());
        let reputation = Reputation::new(
            MIN_INCLUSION_RATE_DENOMINATOR,
            THROTTLING_SLACK,
            BAN_SLACK,
            U256::from(1),
            U256::from(0),
            HashSet::<Address>::default(),
            HashSet::<Address>::default(),
            entry,
        );
        reputation_test_case(reputation);
    }
}
