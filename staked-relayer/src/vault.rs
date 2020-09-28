use crate::Error;
use crate::{
    bitcoin,
    bitcoin::{BitcoinMonitor, BlockHash, GetRawTransactionResult, Txid},
};
use futures::stream::iter;
use futures::stream::StreamExt;
use log::{error, info};
use runtime::{AccountId, H256Le, PolkaBtcVault, StakedRelayerPallet};
use sp_core::H160;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Vaults(RwLock<HashMap<H160, PolkaBtcVault>>);

impl Vaults {
    pub fn from(vaults: HashMap<H160, PolkaBtcVault>) -> Self {
        Self(RwLock::new(vaults))
    }

    pub async fn write(&self, key: H160, value: PolkaBtcVault) {
        self.0.write().await.insert(key, value);
    }

    pub async fn contains_key(&self, addr: H160) -> Option<AccountId> {
        let vaults = self.0.read().await;
        if let Some(vault) = vaults.get(&addr.clone()) {
            return Some(vault.id.clone());
        }
        None
    }
}

pub struct VaultsMonitor<P: StakedRelayerPallet> {
    btc_height: u32,
    btc_rpc: Arc<BitcoinMonitor>,
    vaults: Arc<Vaults>,
    polka_rpc: Arc<P>,
}

impl<P: StakedRelayerPallet> VaultsMonitor<P> {
    pub fn new(
        btc_height: u32,
        btc_rpc: Arc<BitcoinMonitor>,
        vaults: Arc<Vaults>,
        polka_rpc: Arc<P>,
    ) -> Self {
        Self {
            btc_height,
            btc_rpc,
            vaults,
            polka_rpc,
        }
    }

    fn get_raw_tx_and_proof(
        &self,
        tx_id: Txid,
        hash: &BlockHash,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let raw_tx = self.btc_rpc.get_raw_tx(&tx_id, hash)?;
        let proof = self.btc_rpc.get_proof(tx_id, hash)?;
        Ok((raw_tx, proof))
    }

    async fn report_invalid(
        &self,
        vault_id: AccountId,
        tx_id: &Txid,
        raw_tx: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<(), Error> {
        info!("Found tx from vault {}", vault_id);
        // check if matching redeem or replace request
        if self
            .polka_rpc
            .is_transaction_invalid(vault_id.clone(), raw_tx.clone())
            .await?
        {
            info!("Transaction is invalid");
            self.polka_rpc
                .report_vault_theft(
                    vault_id,
                    H256Le::from_bytes_le(&tx_id.as_hash()),
                    self.btc_height,
                    proof,
                    raw_tx,
                )
                .await?;
        }

        Ok(())
    }

    async fn check_transaction(
        &self,
        tx: GetRawTransactionResult,
        block_hash: BlockHash,
    ) -> Result<(), Error> {
        let tx_id = tx.txid;

        // TODO: spawn_blocking?
        let (raw_tx, proof) = self.get_raw_tx_and_proof(tx_id.clone(), &block_hash)?;

        let addresses = bitcoin::extract_btc_addresses(tx);
        let vault_ids = filter_matching_vaults(addresses, &self.vaults).await;

        for vault_id in vault_ids {
            self.report_invalid(vault_id, &tx_id, raw_tx.clone(), proof.clone())
                .await?;
        }

        Ok(())
    }

    async fn scan_next_height(&mut self) -> Result<(), Error> {
        info!("Scanning height {}", self.btc_height);
        let block_hash = self.btc_rpc.wait_for_block(self.btc_height).await?;
        for maybe_tx in self.btc_rpc.get_block_transactions(&block_hash)? {
            if let Some(tx) = maybe_tx {
                self.check_transaction(tx, block_hash).await?
            }
        }
        self.btc_height += 1;
        Ok(())
    }

    pub async fn scan(&mut self) {
        loop {
            if let Err(err) = self.scan_next_height().await {
                error!("Something went wrong: {}", err.to_string());
            }
        }
    }
}

async fn filter_matching_vaults(addresses: Vec<H160>, vaults: &Vaults) -> Vec<AccountId> {
    iter(addresses)
        .filter_map(|addr| vaults.contains_key(addr))
        .collect::<Vec<AccountId>>()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use runtime::PolkaBtcStatusUpdate;
    use runtime::{AccountId, Error, ErrorCode, H256Le, StatusCode};
    use sp_core::U256;
    use sp_keyring::AccountKeyring;

    mockall::mock! {
        Provider {}

        #[async_trait]
        trait StakedRelayerPallet {
            async fn register_staked_relayer(&self, stake: u128) -> Result<(), Error>;
            async fn deregister_staked_relayer(&self) -> Result<(), Error>;
            async fn suggest_status_update(
                &self,
                deposit: u128,
                status_code: StatusCode,
                add_error: Option<ErrorCode>,
                remove_error: Option<ErrorCode>,
                block_hash: Option<H256Le>,
            ) -> Result<(), Error>;
            async fn vote_on_status_update(
                &self,
                status_update_id: U256,
                approve: bool,
            ) -> Result<(), Error>;
            async fn get_status_update(&self, id: u64) -> Result<PolkaBtcStatusUpdate, Error>;
            async fn report_oracle_offline(&self) -> Result<(), Error>;
            async fn report_vault_theft(
                &self,
                vault_id: AccountId,
                tx_id: H256Le,
                tx_block_height: u32,
                merkle_proof: Vec<u8>,
                raw_tx: Vec<u8>,
            ) -> Result<(), Error>;
            async fn is_transaction_invalid(
                &self,
                vault_id: AccountId,
                raw_tx: Vec<u8>,
            ) -> Result<bool, Error>;
        }
    }

    #[tokio::test]
    async fn test_filter_matching_vaults() {
        let mut vault = PolkaBtcVault::default();
        vault.id = AccountKeyring::Bob.to_account_id();
        let vaults = Vaults::from(
            vec![(H160::from_slice(&[0; 20]), vault)]
                .into_iter()
                .collect(),
        );

        assert_eq!(
            filter_matching_vaults(vec![H160::from_slice(&[0; 20])], &vaults).await,
            vec![AccountKeyring::Bob.to_account_id()],
        );

        assert_eq!(
            filter_matching_vaults(vec![H160::from_slice(&[1; 20])], &vaults).await,
            vec![],
        );
    }

    #[tokio::test]
    async fn test_report_valid_transaction() {
        let mut prov = MockProvider::default();
        prov.expect_is_transaction_invalid()
            .returning(|_, _| Ok(false));
        prov.expect_report_vault_theft()
            .never()
            .returning(|_, _, _, _, _| Ok(()));

        let monitor = VaultsMonitor::new(
            0,
            Arc::new(bitcoin::BitcoinMonitor::new(
                bitcoin::bitcoin_rpc_from_env().unwrap(),
            )),
            Arc::new(Vaults::default()),
            Arc::new(prov),
        );

        monitor
            .report_invalid(
                AccountKeyring::Bob.to_account_id(),
                &Txid::default(),
                vec![],
                vec![],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_report_invalid_transaction() {
        let mut prov = MockProvider::default();
        prov.expect_is_transaction_invalid()
            .returning(|_, _| Ok(true));
        prov.expect_report_vault_theft()
            .once()
            .returning(|_, _, _, _, _| Ok(()));

        let monitor = VaultsMonitor::new(
            0,
            Arc::new(bitcoin::BitcoinMonitor::new(
                bitcoin::bitcoin_rpc_from_env().unwrap(),
            )),
            Arc::new(Vaults::default()),
            Arc::new(prov),
        );

        monitor
            .report_invalid(
                AccountKeyring::Bob.to_account_id(),
                &Txid::default(),
                vec![],
                vec![],
            )
            .await
            .unwrap();
    }
}