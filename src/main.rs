#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate clap;
extern crate gfapi_sys;
//#[macro_use]
//extern crate log;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use std::fs::create_dir;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use clap::{Arg, App};
use gfapi_sys::gluster::*;
use rocket_contrib::Json;
use rocket::{Request, State};
use rocket::config::{Config, Environment};
use rocket::response::status::{Accepted, Created};

#[cfg(test)]
mod tests;

type MessageMap = Mutex<HashMap<String, String>>;

#[derive(Debug, Deserialize)]
struct JsonOptions {
    #[serde(rename = "kubernetes.io/fsType")]
    fs_type: String,
    #[serde(rename = "kubernetes.io/readwrite")]
    readwrite: String,
    #[serde(rename = "kubernetes.io/secret/key1")]
    secret_key: String,
}

#[derive(Debug, Serialize)]
struct GlusterClusters {
    id: String,
    nodes: Vec<String>,
    volumes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct VolumeInfo {
    name: String,
    id: String,
    cluster: String,
    size: u64,
    durability: Durability,
    snapshot: Snapshot,
    mount: Mount,
    bricks: Vec<Brick>,
}

#[derive(Debug, Deserialize)]
struct ExpandVolumeRequest {
    /// Size in GB
    expand_size: u64,
}

#[derive(Debug, Deserialize)]
struct CreateVolumeRequest {
    /// Size in GB
    size: u64,
    /// Name of volume. If not provided, the name of the volume
    /// will be vol_{id}
    name: Option<String>,
    durability: Option<Durability>,
    snapshot: Snapshot,
    clusters: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Serialize)]
pub enum VolumeType {
    #[serde(rename = "replicate")]
    Replicate { replica: Option<u8> }, // defaults to 2 },
    #[serde(rename = "disperse")]
    Disperse {
        data: Option<u8>, // defaults to 8
        redundancy: Option<u8>, // defaults to 2
    },
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Deserialize, Serialize)]
struct Durability {
    #[serde(rename = "type")]
    mount_type: VolumeType,
    replicate: HashMap<String, u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Snapshot {
    /// Defaults to false
    enable: Option<bool>,
    /// Defaults to 1.5
    factor: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Mount {
    glusterfs: GlusterFsMount,
}

#[derive(Debug, Serialize)]
struct GlusterFsMount {
    device: String,
    options: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct Brick {
    id: String,
    path: PathBuf, //"/gluster/brick_aaaaaad2e40df882180479024ac4c24c8/brick",
    size: u64,
    node: String,
    device: String,
}

#[derive(Debug, Serialize)]
struct VolumeList {
    volumes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ClusterList {
    clusters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddNodeRequest {
    zone: u64,
    hostnames: ManagedHosts,
    storage: Vec<String>,
    cluster: String,
}

#[derive(Debug, Deserialize)]
struct ManagedHosts {
    manage: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddDeviceRequest {
    node: String, //": "714c510140c20e808002f2b074bc0c50",
    name: PathBuf, //": "/dev/sdb"
}

#[derive(Debug, Serialize)]
struct DeviceInfo {
    name: PathBuf, //": "/dev/sdh",
    storage: Storage,
    id: String,
    bricks: Vec<Brick>,
}

#[derive(Debug, Serialize)]
struct Storage {
    total: u64,
    free: u64,
    used: u64,
}

#[post("/clusters", format = "application/json")]
fn create_cluster(map: State<MessageMap>) -> Created<Json<GlusterClusters>> {
    let clusters = GlusterClusters {
        id: "".to_string(),
        nodes: vec![],
        volumes: vec![],
    };

    //let cluster = Gluster::connect("volume", "localhost", 24007);

    let mut hashmap = map.lock().expect("map lock.");
    //    if hashmap.contains_key(&id) {
    //        json!({
    //            "status": "error",
    //            "reason": "ID exists. Try put."
    //        })
    //    } else {
    //
    //    }
    Created("".to_string(), Some(Json(clusters)))
}

#[get("/clusters/<id>", format = "application/json")]
fn get_cluster_info(id: String, map: State<MessageMap>) -> Json<GlusterClusters> {
    let clusters = GlusterClusters {
        id: "".to_string(),
        nodes: vec![],
        volumes: vec![],
    };

    Json(clusters)
}

#[get("/clusters", format = "application/json")]
fn list_clusters(map: State<MessageMap>) -> Json<ClusterList> {
    let clusters = ClusterList { clusters: vec![] };
    Json(clusters)
}

#[delete("/clusters/<id>", format = "application/json")]
fn delete_cluster(id: String, map: State<MessageMap>) {
    //json!({ "status": "ok" })
}

#[post("/volumes", format = "application/json", data = "<input>")]
fn create_volume(input: Json<CreateVolumeRequest>, map: State<MessageMap>) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[get("/nodes/<id>", format = "application/json")]
fn get_node_info(id: String) -> Json<String> {
    Json("".into())
}

#[delete("/nodes/<id>", format = "application/json")]
fn delete_node(id: String) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[post("/nodes", format = "application/json", data = "<input>")]
fn add_node(input: Json<AddNodeRequest>) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[post("/devices", format = "application/json", data = "<input>")]
fn add_device(input: Json<AddDeviceRequest>) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[delete("/devices/<id>", format = "application/json")]
fn delete_device(id: String) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[get("/devices/<id>", format = "application/json")]
fn get_device_info(id: String) -> Json<DeviceInfo> {
    let device_info = DeviceInfo {
        name: PathBuf::from("/dev/sda"), //": "/dev/sdh",
        storage: Storage {
            total: 0,
            free: 0,
            used: 0,
        },
        id: "".into(),
        bricks: vec![],
    };
    Json(device_info)
}

#[get("/volumes/<id>", format = "application/json")]
fn get_volume_info(id: String, map: State<MessageMap>) -> Json<VolumeInfo> {
    let volume_info = VolumeInfo {
        name: "".into(),
        id: "".into(),
        cluster: "".into(),
        size: 0,
        durability: Durability {
            mount_type: VolumeType::Replicate { replica: Some(3) },
            replicate: HashMap::new(),
        },
        snapshot: Snapshot {
            enable: Some(true),
            factor: Some(0.00),
        },
        mount: Mount {
            glusterfs: GlusterFsMount {
                device: "".into(),
                options: HashMap::new(),
            },
        },
        bricks: vec![],
    };
    Json(volume_info)
}

#[post("/volumes/<id>/expand", format = "application/json", data = "<input>")]
fn expand_volume(
    id: String,
    input: Json<ExpandVolumeRequest>,
    map: State<MessageMap>,
) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[delete("/volumes/<id>", format = "application/json")]
fn delete_volume(id: String, map: State<MessageMap>) -> Accepted<String> {
    Accepted(Some("".into()))
}

#[get("/volumes", format = "application/json")]
fn list_volumes(map: State<MessageMap>) -> Json<VolumeList> {
    let volumes = VolumeList { volumes: vec![] };
    Json(volumes)
}

#[error(500)]
fn internal_error() -> &'static str {
    "Whoops! Looks like we messed up."
}

#[error(400)]
fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
}

// TODO: gluster_host and volume go away and should come from json_options
/*
fn do_mount(
    dir: &str,
    gluster_host: &str,
    volume: &str,
    json_options: Option<&str>,
) -> Result<(), GlusterError> {
    println!("json_options: {:?}", json_options);
    let cluster = Gluster::connect(volume, gluster_host, 24007)?;
    let dir_path = Path::new(dir);
    // Create the mount point on the cluster
    if !cluster.exists(&dir_path)? {
        cluster.mkdir(&dir_path, 0644)?;
    }
    // Create the mount point on the host
    if !dir_path.exists() {
        create_dir(&dir_path)?;
    }
    let mount_cmd = Command::new("mount")
        .args(
            &[
                "-t",
                "glusterfs",
                &format!("{}:/{}/{}", gluster_host, volume, dir),
                dir,
            ],
        )
        .output()?;
    if !mount_cmd.status.success() {
        return Err(GlusterError::Error(
            String::from_utf8_lossy(&mount_cmd.stdout).into_owned(),
        ));
    }
    Ok(())
}

fn do_unmount(dir: &str) -> Result<(), GlusterError> {
    let mount_cmd = Command::new("unmount").arg(dir).output()?;
    if !mount_cmd.status.success() {
        return Err(GlusterError::Error(
            String::from_utf8_lossy(&mount_cmd.stdout).into_owned(),
        ));
    }
    Ok(())
}

fn process_cmd(res: Result<(), GlusterError>) {
    match res {
        Ok(_) => {
            println!("{}", json!({"status": "Success"}));
        }
        Err(e) => {
            println!("{}", json!({"status": "Failure", "message": e.to_string()}));
        }
    }
}
*/

fn gluster_conn(vol: &str) -> Gluster {
    Gluster::connect(vol, "localhost", 24007).unwrap()
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount(
            "/",
            routes![
                get_cluster_info,
                create_cluster,
                list_clusters,
                delete_cluster,
                get_volume_info,
                list_volumes,
                create_volume,
                expand_volume,
                delete_volume,
                get_node_info,
                add_node,
                delete_node,
                get_device_info,
                add_device,
                delete_device,
            ],
        )
        .catch(errors![internal_error, not_found])
        .manage(Mutex::new(HashMap::<String, String>::new()))
}

fn main() {
    let matches = App::new("gluster-flexvol")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Gluster thin Kubernetes volumes")
        .arg(
            Arg::with_name("volume")
                .long("volume")
                .help("The gluster volume to manage")
                .required(true)
                .takes_value(true),
        )
        .get_matches();


    rocket()
        //.manage(gluster_conn(matches.value_of("volume").unwrap()))
        .launch();
}
