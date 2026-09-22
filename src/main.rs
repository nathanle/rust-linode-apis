use awc::{Client, http::header::HeaderName, http::header::HeaderValue};
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::io::Bytes;
use std::env;
use std::any::{Any, TypeId};

const API_VERSION: &str = "v4";

#[derive(Serialize, Deserialize, Debug)]
struct LinodeVMObject {
    alerts: LinodeObject,
    backups: BackUps,
    #[serde(default)]
    capabilities: Vec<String>,
    disk_encryption: String,
    group: String,
    has_user_data: bool,
    host_uuid: String,
    hypervisor: String,
    id: i32,
    image: String,
    interface_generation: String,
    ipv4: Vec<String>,
    ipv6: String,
    label: String,
    lke_cluster_id: Option<i32>,
    maintenance_policy: String,
    placement_group: Option<String>,
    region: String,
    site_type: String,
    specs: Specs,
    status: String,
    tags: Vec<String>,
    r#type: String,
    updated: String,
    watchdog_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct BackUps {
    available: bool,
    enabled: bool,
    last_successful: Option<String>,
    schedule: Schedule,
}

#[derive(Serialize, Deserialize, Debug)]
struct Schedule {
    day: Option<String>,
    window: Option<String>,

}
#[derive(Serialize, Deserialize, Debug)]
struct LinodeObject {
    cpu: i32,
    io: i32,
    network_in: i32,
    network_out: i32,
    transfer_quota: i32,
}
#[derive(Serialize, Deserialize, Debug)]
struct DataPrices {
    id: String,
    label: String,
    price: Prices,
    region_prices: Vec<i32>,
    transfer: i32
}

#[derive(Serialize, Deserialize, Debug)]
struct Prices {
    hourly: Option<f32>,
    monthly: Option<f32> 
}

#[derive(Serialize, Deserialize, Debug)]
struct Specs {
    accelerated_devices: i32,
    disk: i32,
    gpus: i32,
    memory: i32,
    transfer: i32,
    vcpus: i32
}

async fn data_prices(client: Client) -> DataPrices {
    let url = format!("https://api.linode.com/{API_VERSION}/network-transfer/prices");
    let mut response = client.get(url).send().await;
    let body = response.expect("REASON").body().await.unwrap();
    let mut object: Value = serde_json::from_slice(&body).unwrap();
    let prices: DataPrices = serde_json::from_value(object["data"][0].clone()).unwrap();
    
    prices
}

async fn list_linodes(client: Client, auth_header: (HeaderName, String)) {
    let url = format!("https://api.linode.com/{API_VERSION}/linode/instances");
    let accept_header = ("accept", "application/json");
    
    let response = client.get(url)
        .insert_header(accept_header)
        .insert_header(auth_header)
        .send()
        .await;

    match response {
        Ok(mut res) => {
            let body = res.body().await.expect("Failed to read body");
            
            // 1. Parse into a generic Value first
            let mut object: Value = serde_json::from_slice(&body).expect("Failed to parse JSON");

            // 2. PRINT THE PAYLOAD HERE
            // This will print the entire JSON structure in a readable format
            //println!("DEBUG PAYLOAD: {}", serde_json::to_string_pretty(&object).unwrap());

            // 3. Now try to deserialize
            let vms: Vec<LinodeVMObject> = serde_json::from_value(object["data"].clone())
                .expect("Failed to deserialize LinodeVMObjects");

            for vm in vms {
                println!("ID: {}, Label: {}", vm.id, vm.label);
            }
        }
        Err(e) => eprintln!("Request failed: {}", e),
    }
}

#[actix_rt::main]
async fn main() {
    let linode_pat = match env::var("LINODE_RUST_PAT") {
        Ok(val) => val,
        Err(e) => panic!("{}", e),
    };
    let headerName = HeaderName::from_static("authorization");
    let authorization = format!("Bearer {linode_pat}");
    let auth_header = (headerName, authorization);
    let client = Client::default();
    //let prices = data_prices(client).await;
    let list_linodes = list_linodes(client, auth_header).await;
    
    //println!("{:#?}", prices);

}
