use sp_keyring::AccountKeyring;
use std::thread;
use std::time::Duration;
use utils::tx_manager::{SystemTokenId, TxManager};

#[tokio::main]
async fn main() {
    // Alice in infra-asset-system -> Bob in parachain-template-node
    {
        let (ws_port, account) = (vec!["http://localhost:9501"], AccountKeyring::Alice);
        let hex_str = "1f0901010100411f01000101008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4801040000020432058d01000f0080c6a47e8d030000000000";
        let system_token_id = SystemTokenId {
            para_id: 1000,
            pallet_id: 50,
            asset_id: 99,
        };
        xcm_transfer(ws_port, account, hex_str, system_token_id).await;
    }

    println!("wait for 12 seconds for transacting xcm ");
    thread::sleep(Duration::from_secs(12));

    // Bob in parachain-template-node -> Alice in infra-asset-system
    {
        let (ws_port, account) = (vec!["http://localhost:9601"], AccountKeyring::Bob);
        let hex_str = "1f0901010100a10f010001010090b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22010400010300a10f0432058d01000f00404c948b32030000000000";
        let system_token_id = SystemTokenId {
            para_id: 2000,
            pallet_id: 12,
            asset_id: 99,
        };
        xcm_transfer(ws_port, account, hex_str, system_token_id).await;
    }
}

async fn xcm_transfer(
    ws_port: Vec<&str>,
    account: AccountKeyring,
    hex_str: &str,
    system_token_id: SystemTokenId,
) {
    let encoded_data = hex::decode(hex_str).expect("should be decoded");
    for url in ws_port {
        let tx_manager = TxManager::new(url.to_string());
        tx_manager
            .send_xcm(encoded_data.clone(), account, system_token_id.clone())
            .await;
    }
}

// // Alice in infra-asset-system -> Bob in parachain-template-node
// async fn xcm_transfer_1(para_id: u32, pallet_id: u32, asset_id: u32) {
//     let hexa_str = "1f0901010100411f01000101008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4801040000020432058d01000f0080c6a47e8d030000000000";
//     let encoded_data = hex::decode(hexa_str).expect("should be decoded");

//     let urls = vec!["http://localhost:9501"];

//     for url in urls {
//         let tx_manager = TxManager::new(url.to_string());
//         tx_manager
//             .send_xcm(
//                 encoded_data.clone(),
//                 AccountKeyring::Alice,
//                 para_id,
//                 pallet_id,
//                 asset_id,
//             )
//             .await;
//     }
// }

// // Bob in parachain-template-node -> Alice in infra-asset-system
// async fn xcm_transfer_2(para_id: u32, pallet_id: u32, asset_id: u32) {
//     let hexa_str = "1f0901010100a10f010001010090b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22010400010300a10f0432058d01000f00404c948b32030000000000";
//     let encoded_data = hex::decode(hexa_str).expect("should be decoded");

//     let urls = vec!["http://localhost:9601"];

//     for url in urls {
//         let tx_manager = TxManager::new(url.to_string());
//         tx_manager
//             .send_xcm(
//                 encoded_data.clone(),
//                 AccountKeyring::Bob,
//                 para_id,
//                 pallet_id,
//                 asset_id,
//             )
//             .await;
//     }
// }
