use awc::{Client, http::header::HeaderName, http::header::HeaderValue};
use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::io::Bytes;
use std::env;

const API_VERSION: &str = "v4";

//Object {"alerts": Object {"cpu": Number(180), "io": Number(10000), 
//"network_in": Number(10), "network_out": Number(10), 
//"transfer_quota": Number(80)}, "backups": Object {"available": Bool(false), 
//"enabled": Bool(false), "last_successful": Null, "schedule": 
//Object {"day": Null, "window": Null}}, "capabilities": 
//Array [String("Block Storage Encryption"), String("SMTP Enabled"), 
//String("Maintenance Policy")], "created": String("2023-07-01T17:08:52"), 
//"disk_encryption": String("disabled"), "group": String(""), 
//"has_user_data": Bool(false), 
//"host_uuid": String("10bbee6d80a94bfae3c777a41d6c416af4a53e8f"),
//"hypervisor": String("kvm"), "id": Number(47397626),
//"image": String("linode/debian12"),
//"interface_generation": String("legacy_config"),
//"ipv4": Array [String("170.187.153.238"), String("192.168.146.93")],
//"ipv6": String("2600:3c02::f03c:93ff:fe8d:260d/128"),
//"label": String("pihole"), "lke_cluster_id": Null,
//"maintenance_policy": String("linode/migrate"),
//"placement_group": Null, "region": String("us-southeast"),
//"site_type": String("core"), "specs": Object {"accelerated_devices": Number(0),
//"disk": Number(81920), "gpus": Number(0), "memory": Number(4096),
//"transfer": Number(4000), "vcpus": Number(2)}, "status": String("running"),
//"tags": Array [String("dns")], "type": String("g6-standard-2"),
//"updated": String("2026-07-03T04:29:17"), "watchdog_enabled": Bool(true)}

#[derive(Serialize, Deserialize, Debug)]
struct LinodeVMObject {
    alerts: LinodeObject,
    backups: BackUps,
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
    lke_cluster_id: Option<String>,
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

async fn list_linodes(client: Client, auth_header: (HeaderName, std::string::String)) {
    let url = format!("https://api.linode.com/{API_VERSION}/linode/instances");
    let header_two = ("accept", "application/json");
    let response = client.get(url)
        .insert_header(header_two)
        .insert_header(auth_header)
        .send()
        .await;
    let body = response.expect("REASON").body().await.unwrap();
    let mut object: Value = serde_json::from_slice(&body).unwrap();
    let vms: LinodeVMObject = serde_json::from_value(object["data"][0].clone()).expect("REASON");
    print!("{:?}", vms);
    //let prices: DataPrices = serde_json::from_value(object["data"][0].clone()).unwrap();
    
    //prices
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
