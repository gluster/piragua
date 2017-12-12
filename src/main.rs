#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate base64;
#[macro_use]
extern crate clap;
extern crate gfapi_sys;
extern crate gluster;
extern crate itertools;
extern crate jsonwebtoken;
extern crate libc;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate uuid;

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Cursor, Error, ErrorKind};
use std::io::Result as IOResult;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;

use base64::decode as base64_decode;
use clap::{Arg, App};
use gfapi_sys::gluster::*;
use gluster::get_local_ip;
use gluster::peer::peer_list;
use gluster::volume::volume_add_quota;
use itertools::Itertools;
use jsonwebtoken::{Algorithm, decode, Validation};
use libc::{S_IRGRP, S_IWGRP, S_IXGRP, S_IRWXU, S_IRUSR, S_IXUSR};
use rocket_contrib::Json;
use rocket::{Outcome, Request, Response, State};
use rocket::http::{ContentType, Status};
use rocket::http::hyper::header::Location;
use rocket::request::{self, FromRequest};
use rocket::response::status::Created;
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
    //disperse: Option<DisperseDurability>,
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
    hosts: Vec<String>,
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
    hostnames: ManagedHosts,
    cluster: String,
    id: Uuid,
    state: String,
    devices: Vec<DeviceInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    iat: u64,
    exp: u64,
    qsh: String,
}

// Json Web Token
struct Jwt(Claims);

impl<'a, 'r> FromRequest<'a, 'r> for Jwt {
    type Error = String;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Jwt, Self::Error> {
        let secret = match env::var("JWT_SECRET") {
            Ok(s) => s,
            Err(e) => {
                return Outcome::Failure((Status::PreconditionFailed, e.to_string()));
            }
        };
        let secret_decoded = match base64_decode(&secret) {
            Ok(s) => s,
            Err(e) => {
                println!("Secret decoding failed: {:?}", e);
                return Outcome::Failure((Status::PreconditionFailed, e.to_string()));
            }
        };
        let token_header = request.headers().get_one("Authorization");
        match token_header {
            Some(auth_token) => {
                // Set the default params for validation
                let mut validate = Validation::default();
                validate.algorithms = Some(vec![Algorithm::HS256]); // set our Algorithm
                validate.leeway = 1000 * 60; // Add 1 minute of leeway for clock skew
                validate.validate_nbf = false;

                let auth_parts: Vec<&str> = auth_token.split_whitespace().collect();
                let token_data =
                    match decode::<Claims>(auth_parts[1], &secret_decoded, &validate) {
                        Ok(data) => data,
                        Err(e) => {
                            println!("jwt decode failed: {:?}", e);
                            return Outcome::Failure((Status::BadRequest, e.to_string()));
                        }
                    };
                return Outcome::Success(Jwt(token_data.claims));
            }
            None => {
                return Outcome::Failure((
                    Status::BadRequest,
                    "JWT token missing from request".into(),
                ));
            }
        };
    }
}



#[post("/clusters", format = "application/json")]
fn create_cluster(web_token: Jwt) -> Created<Json<GlusterClusters>> {
    let clusters = GlusterClusters {
        id: "cluster-test".to_string(),
        nodes: vec![],
        volumes: vec![],
    };

    Created("".to_string(), Some(Json(clusters)))
}

#[get("/clusters/<cluster_id>", format = "application/json")]
fn get_cluster_info(
    web_token: Jwt,
    cluster_id: String,
    state: State<Gluster>,
) -> Result<Json<GlusterClusters>, String> {
    let mut vol_list: Vec<String> = vec![];

    // Get all the peers in the cluster
    let local_uuid = get_local_uuid().map_err(|e| e.to_string())?;
    let mut peer_uuids = get_peer_uuids().map_err(|e| e.to_string())?;
    if let Some(local) = local_uuid {
        peer_uuids.push(local);
    }

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
        // Transform to Vec of String's
        nodes: peer_uuids
            .iter()
            .map(|uuid| uuid.hyphenated().to_string())
            .collect::<Vec<String>>(),
        volumes: vol_list,
    };

    Ok(Json(clusters))
}

#[get("/clusters")]
fn list_clusters(web_token: Jwt, state: State<String>) -> Json<ClusterList> {
    // Only return the single volume as a cluster
    let clusters = ClusterList { clusters: vec![state.inner().clone()] };
    println!(
        "list clusters: {}",
        serde_json::to_string(&clusters).unwrap()
    );
    Json(clusters)
}

#[delete("/clusters/<id>", format = "application/json")]
fn delete_cluster(web_token: Jwt, id: String) {
    //json!({ "status": "ok" })
}

#[get("/nodes/<id>", format = "application/json")]
fn get_node_info(web_token: Jwt, id: String) -> Result<Json<NodeInfoResponse>, String> {
    // heketi thinks this is a mgmt node
    // get info on 192.168.1.2
    let node_uuid = Uuid::from_str(&id).map_err(|e| e.to_string())?;
    let local_uuid = get_local_uuid().map_err(|e| e.to_string())?;
    let host_ip: IpAddr;

    match local_uuid {
        Some(local) => {
            //is this my local uuid?
            if local == node_uuid {
                host_ip = get_local_ip().map_err(|e| e.to_string())?;
            } else {
                // It's not so lets see if it's one of my peers
                host_ip = match get_peer_info(&node_uuid).map_err(|e| e.to_string())? {
                    Some(ip) => ip,
                    None => {
                        //It's not my local or a peer.  I don't know what this is
                        println!("get_node_info discovery failed for: {}", id);
                        return Err(format!("Unable to find info for {}", id));
                    }
                };
            }
        }
        None => {
            // I can't find my local uuid so fail. Is gluster not running?
            return Err("Unable to find local gluster uuid".to_string());
        }
    }

    let resp = NodeInfoResponse {
        zone: 1,
        id: node_uuid,
        cluster: "cluster-test".into(),
        hostnames: ManagedHosts {
            // Everyone manages themselves
            manage: vec![host_ip.to_string()],
            storage: vec![host_ip.to_string()],
        },
        devices: vec![],
        state: "online".into(),
    };
    println!(
        "node info response: {}",
        serde_json::to_string(&resp).unwrap()
    );
    Ok(Json(resp))
}

#[delete("/nodes/<id>", format = "application/json")]
fn delete_node<'a>(web_token: Jwt, id: String) -> Result<Response<'a>, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Volume created"));
    Ok(response)
}

#[post("/nodes", format = "application/json", data = "<input>")]
fn add_node<'a>(web_token: Jwt, input: Json<AddNodeRequest>) -> Result<Response<'a>, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Node created"));
    Ok(response)
}

#[post("/devices", format = "application/json", data = "<input>")]
fn add_device<'a>(web_token: Jwt, input: Json<AddDeviceRequest>) -> Result<Response<'a>, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Device created"));
    Ok(response)
}

#[delete("/devices/<id>", format = "application/json")]
fn delete_device<'a>(web_token: Jwt, id: String) -> Result<Response<'a>, String> {
    //NOPE you're not allowed
    let mut response = Response::new();
    response.set_status(Status::new(204, "Device deleted"));
    Ok(response)
}

#[get("/devices/<device_id>", format = "application/json")]
fn get_device_info(web_token: Jwt, device_id: String) -> Json<DeviceInfo> {
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
    web_token: Jwt,
    input: Json<CreateVolumeRequest>,
    state: State<Gluster>,
    vol_name: State<String>,
) -> Result<Response<'a>, String> {
    println!("volume request: {:#?}", input);

    let vol_id = if input.name == "" {
        let u = Uuid::new_v4();
        u.hyphenated().to_string()
    } else {
        input.name.clone()
    };
    let dir_path = Path::new(&vol_id);

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

    let quota_path = PathBuf::from(format!("/{}", vol_id));
    println!(
        "Adding {}GB sized quota to: {}",
        input.size,
        quota_path.display()
    );
    match volume_add_quota(&vol_name, &quota_path, input.size * 1024 * 1024 * 1024) {
        Ok(_) => {}
        Err(e) => {
            println!("volume_add_quota_failed: {}", e.to_string());
        }
    }

    let mut response = Response::new();
    response.set_header(Location(format!("/volumes/{}", vol_id)));
    response.set_status(Status::Accepted);

    Ok(response)
}

// List the peer uuids but not the local one.  Use get_local_uuid for that
fn get_peer_uuids() -> IOResult<Vec<Uuid>> {
    let mut uuids: Vec<Uuid> = Vec::new();
    for entry in fs::read_dir(Path::new("/var/lib/glusterd/peers"))? {
        let entry = entry?;
        let u = Uuid::from_str(&entry.file_name().to_string_lossy())
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        uuids.push(u);
    }
    Ok(uuids)
}

// Get the local uuid for this glusterd
fn get_local_uuid() -> IOResult<Option<Uuid>> {
    let f = File::open("/var/lib/glusterd/glusterd.info")?;
    let f = BufReader::new(f);
    for line in f.lines() {
        let l = line?;
        if l.starts_with("UUID") {
            let l = l.replace("UUID=", "");
            let guid = Uuid::from_str(&l).map_err(|e| {
                Error::new(ErrorKind::Other, e.to_string())
            })?;
            return Ok(Some(guid));
        }
    }
    Ok(None)
}

// Get the gluster peer ip address
fn get_peer_info(uuid: &Uuid) -> IOResult<Option<IpAddr>> {
    let f = File::open(format!("/var/lib/glusterd/peers/{}", uuid.hyphenated()))?;
    let f = BufReader::new(f);
    for line in f.lines() {
        let l = line?;
        if l.starts_with("hostname") {
            let l = l.replace("hostname1=", "");
            let ip_addr = IpAddr::from_str(&l).map_err(
                |e| Error::new(ErrorKind::Other, e),
            )?;
            return Ok(Some(ip_addr));
        }
    }
    Ok(None)
}

fn get_gluster_vol(vol_id: &str) -> IOResult<HashMap<String, String>> {
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
fn get_volume_info<'a>(
    web_token: Jwt,
    vol_id: String,
    vol_name: State<String>,
) -> Result<Response<'a>, String> {
    let vol_info = get_gluster_vol(&vol_name).map_err(|e| e.to_string())?;
    let peers = peer_list().map_err(|e| e.to_string())?;

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

    let response_data = VolumeInfo {
        name: format!("{volume}/{path}", volume = *vol_name, path = &vol_id),
        id: vol_id.clone(),
        cluster: "cluster-test".into(),
        // TODO: This should be changed to the quota size
        size: 10,
        durability: Durability {
            mount_type: Some(VolumeType::Replicate),
            replicate: Some(ReplicaDurability { replica: Some(3) }),
        },
        snapshot: Snapshot {
            enable: Some(true),
            factor: Some(1.20),
        },
        mount: Mount {
            glusterfs: GlusterFsMount {
                hosts: backup_servers,
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
    println!(
        "VolumeInfo: {}",
        serde_json::to_string(&response_data).unwrap()
    );
    let response = Response::build()
        .header(ContentType::JSON)
        .raw_header("X-Pending", "false")
        .sized_body(Cursor::new(serde_json::to_string(&response_data).unwrap()))
        .finalize();
    println!("response: {:#?}", response);
    Ok(response)
}

#[post("/volumes/<vol_name>/<id>/expand", format = "application/json", data = "<input>")]
fn expand_volume<'a>(
    web_token: Jwt,
    vol_name: String,
    id: String,
    input: Json<ExpandVolumeRequest>,
) -> Result<Response<'a>, String> {

    let mut response = Response::new();
    response.set_header(Location(format!("/volumes/{}/{}", vol_name, id)));
    response.set_status(Status::Accepted);

    // If this doesn't have a quota already it'll fail to remove
    let quota_path = PathBuf::from(format!("/{}", id));
    // input.expand_size needs to be converted to bytes from GB of input
    volume_add_quota(
        &vol_name,
        &quota_path,
        input.expand_size * 1024 * 1024 * 1024,
    ).map_err(|e| e.to_string())?;

    Ok(response)
}

#[delete("/volumes/<vol_name>/<vol_id>")]
fn delete_volume<'a>(
    web_token: Jwt,
    vol_name: String,
    vol_id: String,
    state: State<Gluster>,
) -> Result<Response<'a>, String> {
    // Clients will keep calling this and we need to return 204 when it's finished
    // This works out well because rm -rf could take awhile.
    let mut response = Response::new();
    response.set_status(Status::Accepted);
    response.set_header(Location(format!("/volumes/{}", vol_id)));

    // Split this into the volume_name/volume_id and just delete the volume_id
    println!("Deleting {}", vol_id);

    // Delete the directory.
    // TODO: How can we background this and tell the client to come back later?
    state.remove_dir_all(&Path::new(&vol_id)).map_err(
        |e| e.to_string(),
    )?;

    Ok(response)
}

#[get("/volumes", format = "application/json")]
fn list_volumes(web_token: Jwt, state: State<Gluster>) -> Result<Json<VolumeList>, String> {
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
