use std::str::FromStr;

use parity_scale_codec::{Compact, Decode, Encode};
use serde_json::{json, Value};
use sp_core::{blake2_256, H256};
use sp_keyring::{sr25519::Keyring, AccountKeyring};
use sp_runtime::{generic::Era, AccountId32, MultiAddress, MultiSignature};
use sp_version::RuntimeVersion;

#[subxt::subxt(runtime_metadata_path = "metadata.scale")]
pub mod relay_chain {}

use relay_chain::runtime_types::xcm::v2::junction::Junction;
use relay_chain::runtime_types::xcm::{
    v2::{
        multiasset::{AssetId, Fungibility, MultiAsset, MultiAssets},
        multilocation::MultiLocation as V2MultiLocation,
    },
    // v3::{
    //     multiasset::{AssetId, Fungibility, MultiAsset, MultiAssets},
    //     multilocation::MultiLocation as V3MultiLocation,
    // },
    VersionedMultiAssets,
    VersionedMultiLocation,
};

use relay_chain::runtime_types::xcm::v2::multilocation::Junctions;

pub fn get_end_points(num: &str) -> Vec<&str> {
    match num {
        "one" => vec!["http://localhost:9501"],
        "two" => vec!["http://localhost:9501", "http://localhost:9601"],
        _ => Default::default(),
    }
}

#[derive(Encode, Decode, Clone)]
pub struct PotVote<AccountId> {
    #[codec(compact)]
    tip: u128,
    asset_id: Option<u32>,
    fee_payer: Option<AccountId>,
    vote_candidate: Option<AccountId>,
}

impl<AccountId> PotVote<AccountId> {
    pub fn new(
        tip: u128,
        asset_id: Option<u32>,
        fee_payer: Option<AccountId>,
        vote_candidate: Option<AccountId>,
    ) -> Self {
        Self {
            tip,
            asset_id,
            fee_payer,
            vote_candidate,
        }
    }
}

pub struct TxManager {
    pub client: reqwest::Client,
    pub url: String,
}

pub struct Signature<Call, Extra, Additional> {
    pub call: Call,
    pub extra: Extra,
    pub additional: Additional,
}

impl<Call: Encode, Extra: Encode, Additional: Encode> Signature<Call, Extra, Additional> {
    fn new(
        keyring: sp_keyring::sr25519::Keyring,
        call: Call,
        extra: Extra,
        additional: Additional,
    ) -> sp_core::sr25519::Signature {
        let full_unsigned_payload = (call, extra, additional);
        let full_unsigned_payload_scale_bytes = full_unsigned_payload.encode();
        if full_unsigned_payload_scale_bytes.len() > 256 {
            keyring.sign(&blake2_256(&full_unsigned_payload_scale_bytes[..]))
        } else {
            keyring.sign(&full_unsigned_payload_scale_bytes)
        }
    }
}

pub struct SignatureXcm<Extra, Additional> {
    pub extra: Extra,
    pub additional: Additional,
}

impl<Extra: Encode, Additional: Encode> SignatureXcm<Extra, Additional> {
    fn new_encoded_data(
        keyring: sp_keyring::sr25519::Keyring,
        mut encoded: Vec<u8>,
        extra: Extra,
        additional: Additional,
    ) -> sp_core::sr25519::Signature {
        let full_unsigned_payload = (extra, additional);
        let full_unsigned_payload_scale_bytes = full_unsigned_payload.encode();
        encoded.extend(full_unsigned_payload_scale_bytes);

        if encoded.len() > 256 {
            keyring.sign(&blake2_256(&encoded[..]))
        } else {
            keyring.sign(&encoded)
        }
    }
}

impl TxManager {
    pub fn new(url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
        }
    }

    async fn request_basic<Params: serde::Serialize + Clone>(
        &self,
        url: &str,
        method: &str,
        params: Params,
    ) -> anyhow::Result<Value> {
        let mut body: Value = self
            .client
            .post(url)
            .json(&json! {{
                // Used to correlate request with response over socket connections.
                // not needed here over our simple HTTP connection, so just set it
                // to 1 always:
                "id": 1,
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            }})
            .send()
            .await?
            .json()
            .await?;

        // take the "result" out of the JSONRPC response:
        Ok(body["result"].take())
    }

    pub async fn send_extrinsic(&self, pallet_index: u8, call_index: u8, caller: &AccountId32) {
        let dest = MultiAddress::Id::<_, u32>(AccountKeyring::Bob.to_account_id());
        let amount = Compact::from(123456789012345u128);
        let caller_nonce = self.get_nonce(caller).await;
        let call = (pallet_index, call_index, dest, amount);
        let runtime_version = self.get_runtime_version().await;
        let genesis_hash = self.get_genesis_hash().await;
        let extra = self.create_extra(caller_nonce, caller);
        let additional = self.create_additional(runtime_version, genesis_hash);
        let sig = self.create_signature(
            sp_keyring::sr25519::Keyring::Alice,
            call.clone(),
            extra.clone(),
            additional,
        );
        let sig_to_encode = Some((
            MultiAddress::Id::<_, u32>(caller),
            MultiSignature::Sr25519(sig),
            extra,
        ));
        let payload_scale_encode = self.encode_extrinsic(sig_to_encode, call);
        let payload_hex = format!("0x{}", hex::encode(payload_scale_encode));

        self.submit_extrinsic(payload_hex).await;
    }

    pub async fn send_xcm(&self, encoded_data: Vec<u8>, account_keyring: Keyring) {
        // let a = sp_keyring::sr25519::Keyring::Alice;
        let caller = &account_keyring.to_account_id();
        let caller_nonce = self.get_nonce(caller).await;
        let runtime_version = self.get_runtime_version().await;
        let genesis_hash = self.get_genesis_hash().await;
        let extra = self.create_extra(caller_nonce, caller);
        let additional = self.create_additional(runtime_version, genesis_hash);
        let sig = self.create_signature_for_xcm(
            account_keyring,
            encoded_data.clone(),
            extra.clone(),
            additional,
        );
        let sig_to_encode = Some((
            MultiAddress::Id::<_, u32>(caller),
            MultiSignature::Sr25519(sig),
            extra,
        ));
        let payload_scale_encode = self.encode_extrinsic_encoded_data(sig_to_encode, encoded_data);
        let payload_hex = format!("0x{}", hex::encode(payload_scale_encode));

        self.submit_extrinsic(payload_hex).await;
    }

    // WIP: should be fixed
    pub async fn send_xcm2(&self, pallet_index: u8, call_index: u8, caller: &AccountId32) {
        let caller_nonce = self.get_nonce(caller).await;
        let parachain_id = 2000u32;

        let dest = VersionedMultiLocation::V2(V2MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::Parachain(parachain_id)),
        });
        let bene_id = AccountKeyring::Bob.to_raw_public();
        let beneficiary = VersionedMultiLocation::V2(V2MultiLocation {
            parents: 0,
            interior: Junctions::X1(Junction::AccountId32 {
                network: relay_chain::runtime_types::xcm::v2::NetworkId::Any,
                id: bene_id,
            }),
        });
        let assets = VersionedMultiAssets::V2(MultiAssets(vec![MultiAsset {
            id: AssetId::Concrete(V2MultiLocation {
                parents: 0,
                interior: Junctions::X2(Junction::PalletInstance(50), Junction::GeneralIndex(99)),
            }),
            fun: Fungibility::Fungible(1000000000000000),
        }]));

        let fee_asset_item = 0;
        let call = (
            pallet_index,
            call_index,
            dest,
            beneficiary,
            assets,
            fee_asset_item,
        );

        let dest2 = VersionedMultiLocation::V2(V2MultiLocation {
            parents: 1,
            interior: Junctions::X1(Junction::Parachain(parachain_id)),
        });
        let bene_id2 = AccountKeyring::Bob.to_raw_public();
        let beneficiary2 = VersionedMultiLocation::V2(V2MultiLocation {
            parents: 0,
            interior: Junctions::X1(Junction::AccountId32 {
                network: relay_chain::runtime_types::xcm::v2::NetworkId::Any,
                id: bene_id2,
            }),
        });
        let assets2 = VersionedMultiAssets::V2(MultiAssets(vec![MultiAsset {
            id: AssetId::Concrete(V2MultiLocation {
                parents: 0,
                interior: Junctions::X2(Junction::PalletInstance(50), Junction::GeneralIndex(99)),
            }),
            fun: Fungibility::Fungible(1000000000000000),
        }]));
        let call2 = (
            pallet_index,
            call_index,
            dest2,
            beneficiary2,
            assets2,
            fee_asset_item,
        );

        println!("call2 : {:?}", call2);

        let runtime_version = self.get_runtime_version().await;
        let genesis_hash = self.get_genesis_hash().await;
        let extra = self.create_extra(caller_nonce, caller);
        let additional = self.create_additional(runtime_version, genesis_hash);
        let sig = self.create_signature(
            sp_keyring::sr25519::Keyring::Alice,
            call,
            extra.clone(),
            additional,
        );
        let sig_to_encode = Some((
            MultiAddress::Id::<_, u32>(caller),
            MultiSignature::Sr25519(sig),
            extra,
        ));
        let payload_scale_encode = self.encode_extrinsic(sig_to_encode, call2);
        let payload_hex = format!("0x{}", hex::encode(payload_scale_encode));

        // encoded_data: 0x1f0901010100411f01000101008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4801040000020432058d01000f0080c6a47e8d030000000000

        self.submit_extrinsic(payload_hex).await;
    }

    async fn submit_extrinsic(&self, payload_hex: String) {
        let res = self
            .request_basic(self.url.as_str(), "author_submitExtrinsic", [payload_hex])
            .await
            .unwrap();
        println!("res: {:?}", res);
    }

    async fn get_nonce(&self, account: &sp_runtime::AccountId32) -> u32 {
        let nonce_json = self
            .request_basic(self.url.as_str(), "system_accountNextIndex", (account,))
            .await
            .unwrap();
        serde_json::from_value(nonce_json).unwrap()
    }

    async fn get_runtime_version(&self) -> RuntimeVersion {
        let runtime_version_json = self
            .request_basic(self.url.as_str(), "state_getRuntimeVersion", ())
            .await
            .unwrap();
        serde_json::from_value(runtime_version_json).unwrap()
    }

    async fn get_genesis_hash(&self) -> H256 {
        let genesis_hash_json = self
            .request_basic(self.url.as_str(), "chain_getBlockHash", [0])
            .await
            .unwrap();
        let genesis_hash_hex = genesis_hash_json.as_str().unwrap();
        H256::from_str(genesis_hash_hex).unwrap()
    }

    fn create_extra(
        &self,
        caller_nonce: u32,
        caller: &AccountId32,
    ) -> (Era, Compact<u32>, PotVote<AccountId32>) {
        (
            Era::Immortal,
            Compact(caller_nonce),
            PotVote::new(
                0,                    // tip
                Some(99),             // asset id '1'
                None,                 // No fee payer
                Some(caller.clone()), // Vote to Alice
            ),
        )
    }

    fn create_additional(
        &self,
        runtime_version: RuntimeVersion,
        genesis_hash: H256,
    ) -> (u32, u32, H256, H256) {
        (
            runtime_version.spec_version,
            runtime_version.transaction_version,
            genesis_hash,
            genesis_hash,
        )
    }

    fn create_signature<Call: Encode, Extra: Encode, Additional: Encode>(
        &self,
        keyring: sp_keyring::sr25519::Keyring,
        call: Call,
        extra: Extra,
        additional: Additional,
    ) -> sp_core::sr25519::Signature {
        Signature::new(keyring, call, extra, additional)
    }

    fn create_signature_for_xcm<Extra: Encode, Additional: Encode>(
        &self,
        keyring: sp_keyring::sr25519::Keyring,
        encoded_data: Vec<u8>,
        extra: Extra,
        additional: Additional,
    ) -> sp_core::sr25519::Signature {
        SignatureXcm::new_encoded_data(keyring, encoded_data, extra, additional)
    }

    fn encode_extrinsic<S: Encode, C: Encode>(&self, sig: Option<S>, call: C) -> Vec<u8> {
        let mut tmp: Vec<u8> = vec![];

        const EXTRINSIC_VERSION: u8 = 4;
        match sig.as_ref() {
            Some(s) => {
                tmp.push(EXTRINSIC_VERSION | 0b1000_0000);
                s.encode_to(&mut tmp);
            }
            None => {
                tmp.push(EXTRINSIC_VERSION & 0b0111_1111);
            }
        }
        call.encode_to(&mut tmp);
        let compact_len = Compact(tmp.len() as u32);
        let mut output: Vec<u8> = vec![];
        compact_len.encode_to(&mut output);
        output.extend(tmp);

        output
    }

    fn encode_extrinsic_encoded_data<S: Encode>(&self, sig: Option<S>, data: Vec<u8>) -> Vec<u8> {
        let mut tmp: Vec<u8> = vec![];

        const EXTRINSIC_VERSION: u8 = 4;
        match sig.as_ref() {
            Some(s) => {
                tmp.push(EXTRINSIC_VERSION | 0b1000_0000);
                s.encode_to(&mut tmp);
            }
            None => {
                tmp.push(EXTRINSIC_VERSION & 0b0111_1111);
            }
        }
        // call.encode_to(&mut tmp);
        tmp.extend(data);
        let compact_len = Compact(tmp.len() as u32);
        let mut output: Vec<u8> = vec![];
        compact_len.encode_to(&mut output);
        output.extend(tmp);

        output
    }
}
