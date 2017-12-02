#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate clap;
extern crate gfapi_sys;
extern crate gluster;
extern crate itertools;
extern crate libc;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use clap::{Arg, App};
use gfapi_sys::gluster::*;
use gluster::peer::peer_list;
use gluster::volume::volume_info;
use itertools::Itertools;
use libc::{S_IRGRP, S_IWGRP, S_IXGRP, S_IRWXU, S_IRUSR, S_IWUSR, S_IXUSR};
use rocket_contrib::Json;
use rocket::{Request, Response, State};
use rocket::http::Status;
use rocket::http::hyper::header::Location;
use rocket::response::status::{Accepted, Created};
use uuid::Uuid;

#[cfg(test)]
mod tests;

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
    clusters: Option<Vec<String>>,
    /// Name of volume. If not provided, the name of the volume
    /// will be vol_{id}
    name: String,
    durability: Option<Durability>,
    gid: u64,
    snapshot: Snapshot,
}

#[derive(Deserialize, Debug, Serialize)]
pub enum VolumeType {
    #[serde(rename = "replicate")]
    Replicate,
    #[serde(rename = "disperse")]
    Disperse,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReplicaDurability {
    replica: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DisperseDurability {
    data: Option<u8>,
    redundancy: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Durability {
    #[serde(rename = "type")]
    mount_type: Option<VolumeType>,
    replicate: Option<ReplicaDurability>,
    diperse: Option<DisperseDurability>,
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
    path: PathBuf,
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

#[derive(Debug, Deserialize, Serialize)]
struct ManagedHosts {
    manage: Vec<String>,
    storage: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddDeviceRequest {
    node: String,
    name: PathBuf,
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

#[derive(Debug, Serialize)]
struct NodeInfoResponse {
    zone: u8,
    id: String,
    cluster: String,
    hostnames: ManagedHosts,
    devices: Vec<DeviceInfo>,
    state: String,
}



#[post("/clusters", format = "application/json")]
fn create_cluster(state: State<Gluster>) -> Created<Json<GlusterClusters>> {
    let clusters = GlusterClusters {
        id: "cluster-test".to_string(),
        nodes: vec![],
        volumes: vec![],
    };

    Created("".to_string(), Some(Json(clusters)))
}

#[get("/clusters/<cluster_id>", format = "application/json")]
fn get_cluster_info(
    cluster_id: String,
    state: State<Gluster>,
) -> Result<Json<GlusterClusters>, String> {
    let mut vol_list: Vec<String> = vec![];

    // Get all the peers in the cluster
    let peers = peer_list().map_err(|e| e.to_string())?;
    let servers: Vec<String> = peers.iter().map(|ref p| p.hostname.clone()).collect();

    //List all the top level directories and return them as volumes
    let d =
        GlusterDirectory { dir_handle: state.opendir(&Path::new("/")).map_err(|e| e.to_string())? };
    for dir_entry in d {
        let dir_name = format!("{}", dir_entry.path.display());
        // Skip the parent and current dir entries
        if dir_name == ".." || dir_name == "." {
            continue;
        }
        vol_list.push(dir_name);
    }

    let clusters = GlusterClusters {
        id: cluster_id,
        nodes: servers,
        volumes: vol_list,
    };

    Ok(Json(clusters))
}

#[get("/clusters", format = "application/json")]
fn list_clusters(state: State<String>) -> Json<ClusterList> {
    // Only return the single volume as a cluster
    let clusters = ClusterList { clusters: vec![state.inner().clone()] };
    Json(clusters)
}

#[delete("/clusters/<id>", format = "application/json")]
fn delete_cluster(id: String, state: State<Gluster>) {
    //json!({ "status": "ok" })
}

#[get("/nodes/<id>", format = "application/json")]
fn get_node_info(id: String, state: State<Gluster>) -> Result<Json<NodeInfoResponse>, String> {
    let hostname = {
        let mut f = File::open("/etc/hostname").map_err(|e| e.to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s.trim().to_string()
    };

    let resp = NodeInfoResponse {
        zone: 1,
        id: id,
        cluster: "cluster-test".into(),
        hostnames: ManagedHosts {
            // Everyone manages themselves
            manage: vec![hostname.clone()],
            storage: vec![hostname],
        },
        devices: vec![],
        state: "online".into(),
    };
    Ok(Json(resp))
}

#[delete("/nodes/<id>", format = "application/json")]
fn delete_node(id: String, state: State<Gluster>) -> Result<Response, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume created"));
    Ok(response)
}

#[post("/nodes", format = "application/json", data = "<input>")]
fn add_node(input: Json<AddNodeRequest>, state: State<Gluster>) -> Result<Response, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume created"));
    Ok(response)
}

#[post("/devices", format = "application/json", data = "<input>")]
fn add_device(input: Json<AddDeviceRequest>, state: State<Gluster>) -> Result<Response, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume created"));
    Ok(response)
}

#[delete("/devices/<id>", format = "application/json")]
fn delete_device(id: String, state: State<Gluster>) -> Result<Response, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume created"));
    Ok(response)
}

#[get("/devices/<device_id>", format = "application/json")]
fn get_device_info(device_id: String, state: State<Gluster>) -> Json<DeviceInfo> {
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

#[post("/volumes", format = "application/json", data = "<input>")]
fn create_volume<'a>(
    input: Json<CreateVolumeRequest>,
    state: State<Gluster>,
) -> Result<Response<'a>, String> {
    println!("volume request: {:#?}", input);
    let vol_name = if input.name == "" {
        let u = Uuid::new_v4();
        format!("vol_{}", u.hyphenated().to_string())
    } else {
        input.name.clone()
    };
    let dir_path = Path::new(&vol_name);

    // Create the mount point on the cluster
    if !state.exists(&dir_path).map_err(|e| e.to_string())? {
        state.mkdir(&dir_path, S_IRWXU).map_err(|e| e.to_string())?;
    }
    // Change the group id on it to match the requested on
    // root and the requesting user can read the directory
    state.chown(&dir_path, 0, input.gid as u32).map_err(
        |e| e.to_string(),
    )?;

    // root can read/execute and requesting user can read/write/execute
    state
        .chmod(&dir_path, S_IRUSR | S_IXUSR | S_IRGRP | S_IWGRP | S_IXGRP)
        .map_err(|e| e.to_string())?;

    let mut response = Response::new();
    response.set_header(Location(format!("/volumes/{}", vol_name)));
    response.set_status(Status::new(303, "Volume created"));

    Ok(response)
}

fn gluster_vols(vol_id: &str) -> IOResult<HashMap<String, String>> {
    let vol_file = File::open(format!("/var/lib/glusterd/vols/{}/info", vol_id))?;
    let mut vol_data = HashMap::new();
    let f = BufReader::new(vol_file);
    for line in f.lines() {
        let l = line?;
        let parts: Vec<&str> = l.split("=").collect();
        if parts.len() != 2 {
            // Skip broken data
            continue;
        }
        vol_data.insert(parts[0].to_string(), parts[1].to_string());
    }
    Ok(vol_data)
}

#[get("/volumes/<vol_id>", format = "application/json")]
fn get_volume_info(
    vol_id: String,
    state: State<Gluster>,
    vol_name: State<String>,
) -> Result<Json<VolumeInfo>, String> {
    // Use this to get the backup-volfile-server info
    //let vol_info = volume_info(&vol_name).map_err(|e| e.to_string())?;
    let vol_info = gluster_vols(&vol_name).map_err(|e| e.to_string())?;
    let peers = peer_list().map_err(|e| e.to_string())?;

    let mut brick_info: Vec<Brick> = Vec::new();
    for item in &vol_info {
        if item.0.starts_with("brick") {
            /*
            brick_info.push(Brick {
                id: "".into(),
                //TODO: what is this supposed to be?
                path: brick.path, //"/gluster/brick_aaaaaad2e40df882180479024ac4c24c8/brick",
                size: 10,
                //TODO: what do I return here for node and device?
                node: "".into(),
                device: "".into(),
            });
            */
        }
    }
    let backup_servers: Vec<String> = peers.iter().map(|ref p| p.hostname.clone()).collect();

    let mut mount_options: HashMap<String, String> = HashMap::new();
    mount_options.insert(
        "backup-volfile-servers".into(),
        backup_servers.iter().join(",").to_string(),
    );

    let response = VolumeInfo {
        name: vol_id.clone(),
        id: vol_info.get("volume-id").unwrap().clone(), //.hyphenated().to_string(),
        cluster: "cluster-test".into(),
        size: 10,
        durability: Durability {
            mount_type: Some(VolumeType::Replicate),
            replicate: Some(ReplicaDurability { replica: Some(3) }),
            diperse: None,
        },
        snapshot: Snapshot {
            enable: Some(true),
            factor: Some(1.20),
        },
        mount: Mount {
            glusterfs: GlusterFsMount {
                device: format!(
                    "{server}:/{volume}/{path}",
                    server = peers[0].hostname,
                    volume = *vol_name,
                    path = vol_id
                ),
                options: mount_options,
            },
        },
        bricks: vec![],
    };
    println!("VolumeInfo: {:#?}", response);
    Ok(Json(response))
}

#[post("/volumes/<id>/expand", format = "application/json", data = "<input>")]
fn expand_volume(id: String, input: Json<ExpandVolumeRequest>, state: State<Gluster>) -> Response {
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume expanded"));
    response
}

#[delete("/volumes/<vol_id>")]
fn delete_volume<'a>(vol_id: String, state: State<Gluster>) -> Result<Response<'a>, String> {
    // Clients will keep calling this and we need to return 204 when it's finished
    // This works out well because rm -rf could take awhile.
    let mut response = Response::new();
    response.set_status(Status::Accepted);
    response.set_header(Location(format!("/volumes/{}", vol_id)));

    // Delete the directory.
    // TODO: How can we background this and tell the client to come back later?
    state.remove_dir_all(&Path::new(&vol_id)).map_err(
        |e| e.to_string(),
    )?;

    Ok(response)
}

#[get("/volumes", format = "application/json")]
fn list_volumes(state: State<Gluster>) -> Result<Json<VolumeList>, String> {
    let mut vol_list: Vec<String> = vec![];
    let d =
        GlusterDirectory { dir_handle: state.opendir(&Path::new("/")).map_err(|e| e.to_string())? };
    for dir_entry in d {
        let dir_name = format!("{}", dir_entry.path.display());
        // Skip the parent and current dir entries
        if dir_name == ".." || dir_name == "." {
            continue;
        }
        vol_list.push(dir_name);
    }
    let volumes = VolumeList { volumes: vol_list };

    Ok(Json(volumes))
}

#[error(500)]
fn internal_error() -> &'static str {
    "Whoops! Looks like we messed up."
}

#[error(400)]
fn not_found(req: &Request) -> String {
    format!("I couldn't find '{}'. Try something else?", req.uri())
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
    // This is safe.  clap enforces that this is required
    let volname = matches.value_of("volume").unwrap();

    println!("Connecting to: gluster vol {}", volname);
    let gluster = match Gluster::connect(volname, "localhost", 24007) {
        Ok(conn) => conn,
        Err(e) => {
            println!("Failed to connect to gluster: {}.  Exiting", e.to_string());
            return;
        }
    };

    rocket()
        .manage(gluster)
        .manage(volname.to_string())
        .launch();
}
