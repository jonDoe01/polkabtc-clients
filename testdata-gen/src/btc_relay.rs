#[path = "param.rs"] mod param;
#[path = "utils.rs"] mod utils;

use sp_core::{H160, H256, U256};
use runtime::{PolkaBtcProvider, Error};
use module_bitcoin::types::*;
use module_bitcoin::formatter::Formattable;

pub struct BtcSimulator {
    prov: PolkaBtcProvider,
    height: u32,
}

impl BtcSimulator {
    pub fn new(relay_prov: PolkaBtcProvider, height: u32) -> Self {
        let prov = relay_prov;
        let height = height;

        Self {prov, height}
    }

    /// Initialize BTC Relay with a generated Bitcoin block
    pub async fn initialize(&mut self) -> Result<Block, Error> {
        let alice_address = H160::from_slice(
            hex::decode(param::ALICE_BTC_ADDRESS)
                    .unwrap()
                    .as_slice(),
            ); 
        let address = Address::from(*alice_address.as_fixed_bytes()); 
        // initialize BTC Relay with one block
        let init_block = BlockBuilder::new()
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588813835)
            .mine(U256::from(2).pow(254.into()));

        let init_block_hash = init_block.header.hash();
        let raw_init_block_header = RawBlockHeader::from_bytes(&init_block.header.format())
            .expect("could not serialize block header");
        
        &self.prov.initialize_btc_relay(raw_init_block_header, self.height).await?;
        println!("Initialized BTC-Relay at height {:?} with hash {:?}", &self.height, init_block_hash);
        
        self.height += 1;
        Ok(init_block)
    }

    /// Generate the Bitcoin transaction and generate blocks according to the
    /// number of required confirmations. Submits these blocks to the BTC-Relay.
    /// Returns the transaction inclusion proof for the transaction.
    pub async fn generate_transaction_and_include(
        &mut self,
        prev_block: &Block,
        btc_address: &str,
        amount: u128, 
        return_data: H256
    ) -> Result<(H256Le, u32, Vec<u8>, Vec<u8>), Error> {
        let dest_address = utils::get_address_from_string(btc_address);
        let address = Address::from(*dest_address.as_fixed_bytes());
        let value = amount as i64;
        let transaction = TransactionBuilder::new()
            .with_version(2)
            .add_input(
                TransactionInputBuilder::new()
                    .with_coinbase(false)
                    .with_previous_hash(prev_block.transactions[0].hash())
                    .build(),
            )
            .add_output(TransactionOutput::p2pkh(value.into(), &address))
            .add_output(TransactionOutput::op_return(0, return_data.as_bytes()))
            .build();

        let block = BlockBuilder::new()
            .with_previous_hash(prev_block.header.hash())
            .with_version(2)
            .with_coinbase(&address, 50, 3)
            .with_timestamp(1588814835)
            .add_transaction(transaction.clone())
            .mine(U256::from(2).pow(254.into()));

        let raw_block_header = RawBlockHeader::from_bytes(&block.header.format())
            .expect("could not serialize block header");

        let tx_id = transaction.tx_id();
        let tx_block_height = self.height;
        let proof = block.merkle_proof(&vec![tx_id]);
        let bytes_proof = proof.format();
        let raw_tx = transaction.format_with(true);

        self.prov.store_block_header(raw_block_header).await?;

        // Mine six new blocks to get over required confirmations
        let mut prev_block_hash = block.header.hash();
        let mut timestamp = 1588814835;
        for _ in 0..param::CONFIRMATIONS {
            self.height += 1;
            timestamp += 1000;
            let conf_block = BlockBuilder::new()
                .with_previous_hash(prev_block_hash)
                .with_version(2)
                .with_coinbase(&address, 50, 3)
                .with_timestamp(timestamp)
                .mine(U256::from(2).pow(254.into()));

            let raw_conf_block_header = RawBlockHeader::from_bytes(&conf_block.header.format())
                .expect("could not serialize block header");
            self.prov.store_block_header(raw_conf_block_header).await?;

            prev_block_hash = conf_block.header.hash();
        }

        Ok((tx_id, tx_block_height, bytes_proof, raw_tx))
    }
}