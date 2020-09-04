use substrate_subxt::Error as XtError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not subscribe to proposals")]
    SubscribeProposals,

    #[error("Could not fetch best block height: {0}")]
    BestBlockHeight(XtError),
    #[error("Could not fetch block hash: {0}")]
    BlockHash(XtError),
    #[error("Could not fetch block header: {0}")]
    BlockHeader(XtError),
    #[error("Could not fetch parachain status: {0}")]
    ParachainStatus(XtError),
    #[error("Could not initialize parachain: {0}")]
    Initialize(XtError),
    #[error("Could not store block header: {0}")]
    StoreBlockHeader(XtError),
    #[error("Could not register staked relayer: {0}")]
    RegisterStakedRelayer(XtError),
    #[error("Could not deregister staked relayer: {0}")]
    DeregisterStakedRelayer(XtError),
    #[error("Could not suggest status update: {0}")]
    SuggestStatusUpdate(XtError),
}
