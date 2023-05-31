use sp_keyring::AccountKeyring;
use utils::tx_manager::{get_end_points, TxManager};

#[tokio::main]
async fn main() {
    let urls = get_end_points("one");
    for url in urls {
        let tx_manager = TxManager::new(url.to_string());
        tx_manager
            .send_extrinsic(10, 0, &AccountKeyring::Alice.to_account_id())
            .await;
    }
}
