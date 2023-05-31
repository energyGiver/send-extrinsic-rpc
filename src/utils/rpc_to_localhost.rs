use serde_json::{json, Value};

pub const URL_1: &str = "http://localhost:9501";
pub const URL_2: &str = "http://localhost:9601";

/// Make an RPC request to the localhost node over HTTP.
pub async fn rpc_to_localhost<Params: serde::Serialize + Clone>(
    method: &str,
    params: Params,
) -> anyhow::Result<Value> {
    rpc(URL_1, method, params.clone()).await
}

/// Make an RPC request to some URL.
pub async fn rpc<Params: serde::Serialize>(
    url: &str,
    method: &str,
    params: Params,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let mut body: Value = client
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

trait RpcParams {
    fn into_params(self) -> Value;
}
