pub mod balances_dot;
pub mod btc_relay;
pub mod collateral;
pub mod exchange_rate_oracle;
pub mod fee;
pub mod frame_system;
pub mod issue;
pub mod redeem;
pub mod refund;
pub mod replace;
pub mod security;
pub mod sla;
pub mod staked_relayers;
pub mod timestamp;
pub mod treasury;
pub mod vault_registry;

pub use btc_relay::{
    BitcoinBlockHeight, BtcAddress, BtcPublicKey, H256Le, RawBlockHeader, RichBlockHeader,
};
pub use security::{ErrorCode, StatusCode};

use parity_scale_codec::{Codec, EncodeLike};
use sp_arithmetic::traits::Saturating;
use sp_runtime::traits::Member;
use substrate_subxt::system::{System, SystemEventsDecoder};
use substrate_subxt_proc_macro::module;

#[module]
pub trait Core: System {
    #[allow(non_camel_case_types)]
    type u64: Codec + EncodeLike + Member + Default;
    #[allow(non_camel_case_types)]
    type u128: Codec + EncodeLike + Member + Default;

    type DOT: Codec + EncodeLike + Member + Default + PartialOrd + Saturating;
    type Balance: Codec + EncodeLike + Member + Default;
    type BTCBalance: Codec + EncodeLike + Member + Default;
    type PolkaBTC: Codec + EncodeLike + Member + Default;
    type RichBlockHeader: Codec + EncodeLike + Member + Default;
    type H256Le: Codec + EncodeLike + Member + Default;
    type H256: Codec + EncodeLike + Member + Default;
    type H160: Codec + EncodeLike + Member + Default;
    type BtcAddress: Codec + EncodeLike + Member + Default;
    type BtcPublicKey: Codec + EncodeLike + Member + Default;

    type ErrorCodes: Codec + EncodeLike + Member + Default;
    type ErrorCode: Codec + EncodeLike + Member + Default;
    type StatusCode: Codec + EncodeLike + Member + Default;

    type SignedFixedPoint: Codec + EncodeLike + Member + Default;
    type UnsignedFixedPoint: Codec + EncodeLike + Member + Default;
}
