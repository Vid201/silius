use crate::{
    mempool::{Mempool, UserOperationAct, UserOperationAddrAct, UserOperationCodeHashAct},
    reputation::{HashSetOp, Reputation, ReputationEntryOp},
    validate::{SanityCheck, SanityHelper},
};
use ethers::{providers::Middleware, types::Address};
use silius_primitives::{
    consts::{
        entities::{FACTORY, PAYMASTER, SENDER},
        reputation::THROTTLED_ENTITY_MEMPOOL_COUNT,
    },
    reputation::{ReputationError, Status},
    sanity::SanityCheckError,
    UserOperation,
};

#[derive(Clone)]
pub struct Entities;

impl Entities {
    /// Gets the status for entity.
    fn get_status<M: Middleware, H, R>(
        &self,
        addr: &Address,
        _helper: &SanityHelper<M>,
        reputation: &Reputation<H, R>,
    ) -> Result<Status, SanityCheckError>
    where
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        Ok(Status::from(reputation.get_status(addr).map_err(|_| {
            SanityCheckError::UnknownError {
                message: "Failed to retrieve reputation status".into(),
            }
        })?))
    }

    /// [SREP-020] - a BANNED address is not allowed into the mempool.
    fn check_banned(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
    ) -> Result<(), SanityCheckError> {
        if *status == Status::BANNED {
            return Err(ReputationError::EntityBanned {
                entity: entity.to_string(),
                address: *addr,
            }
            .into());
        }

        Ok(())
    }

    /// [SREP-030] - THROTTLED address is limited to THROTTLED_ENTITY_MEMPOOL_COUNT entries in the mempool
    fn check_throttled<M: Middleware, T, Y, X, Z, H, R>(
        &self,
        entity: &str,
        addr: &Address,
        status: &Status,
        _helper: &SanityHelper<M>,
        mempool: &Mempool<T, Y, X, Z>,
        _reputation: &Reputation<H, R>,
    ) -> Result<(), SanityCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        if *status == Status::THROTTLED
            && (mempool.get_number_by_sender(addr) + mempool.get_number_by_entity(addr))
                >= THROTTLED_ENTITY_MEMPOOL_COUNT
        {
            return Err(ReputationError::ThrottledLimit {
                entity: entity.to_string(),
                address: *addr,
            }
            .into());
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl<M: Middleware> SanityCheck<M> for Entities {
    /// The [check_user_operation] method implementation that performs the sanity check for the staked entities.
    ///
    /// # Arguments
    /// `uo` - The user operation to be checked.
    /// `helper` - The [sanity check helper](SanityHelper) that contains the necessary data to perform the sanity check.
    ///
    /// # Returns
    /// None if the sanity check is successful, otherwise a [SanityCheckError] is returned.
    async fn check_user_operation<T, Y, X, Z, H, R>(
        &self,
        uo: &UserOperation,
        mempool: &Mempool<T, Y, X, Z>,
        reputation: &Reputation<H, R>,
        helper: &SanityHelper<M>,
    ) -> Result<(), SanityCheckError>
    where
        T: UserOperationAct,
        Y: UserOperationAddrAct,
        X: UserOperationAddrAct,
        Z: UserOperationCodeHashAct,
        H: HashSetOp,
        R: ReputationEntryOp,
    {
        let (sender, factory, paymaster) = uo.get_entities();

        // [SREP-040] - an OK staked entity is unlimited by the reputation rule

        // sender
        let status = self.get_status(&sender, helper, reputation)?;
        self.check_banned(SENDER, &sender, &status)?;
        self.check_throttled(SENDER, &sender, &status, helper, mempool, reputation)?;

        // factory
        if let Some(factory) = factory {
            let status = self.get_status(&factory, helper, reputation)?;
            self.check_banned(FACTORY, &factory, &status)?;
            self.check_throttled(FACTORY, &factory, &status, helper, mempool, reputation)?;
        }

        // paymaster
        if let Some(paymaster) = paymaster {
            let status = self.get_status(&paymaster, helper, reputation)?;
            self.check_banned(PAYMASTER, &paymaster, &status)?;
            self.check_throttled(PAYMASTER, &paymaster, &status, helper, mempool, reputation)?;
        }

        Ok(())
    }
}
