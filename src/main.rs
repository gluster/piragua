#[macro_use]
extern crate clap;
extern crate gfapi_sys;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use std::fs::create_dir;
use std::path::Path;
use std::process::Command;

use clap::{Arg, App, SubCommand};
use gfapi_sys::gluster::*;

#[derive(Deserialize)]
struct JsonOptions {
    #[serde(rename = "kubernetes.io/fsType")]
    fs_type: String,
    #[serde(rename = "kubernetes.io/readwrite")]
    readwrite: String,
    #[serde(rename = "kubernetes.io/secret/key1")]
    secret_key: String,
}

// TODO: gluster_host and volume go away and should come from json_options
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

fn main() {
    let matches = App::new("gluster-flexvol")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Gluster thin Kubernetes volumes")
        .subcommand(SubCommand::with_name("init").about("Initialize the driver"))
        .subcommand(
            SubCommand::with_name("isattached")
                .about("Check if a vol is attached to a host")
                .arg(
                    Arg::with_name("json_options")
                        .help("Json options")
                        .required(false),
                )
                .arg(Arg::with_name("node_name").help("Node name").required(
                    false,
                )),
        )
        .subcommand(
            SubCommand::with_name("mount")
                .about("Mount a vol at a dir")
                .arg(
                    Arg::with_name("mount_dir")
                        .help("mount directory")
                        .required(true),
                )
                .arg(
                    Arg::with_name("json_options")
                        .help("Json options")
                        .required(false),
                ),
        )
        .subcommand(
            SubCommand::with_name("unmount")
                .about("Unmount a vol from a dir")
                .arg(
                    Arg::with_name("mount_dir")
                        .help("mount directory")
                        .required(true),
                ),
        )
        .get_matches();

    //Handle commands
    if matches.subcommand_matches("init").is_some() {
        println!("{}", json!({"status": "success", "capabilities": {"attach": false}}));
    }
    if let Some(ref matches) = matches.subcommand_matches("isattached") {
        //
    }
    if let Some(ref matches) = matches.subcommand_matches("mount") {
        let mount_dir = matches.value_of("mount_dir").unwrap();
        let json_options = matches.value_of("json_options");
        process_cmd(do_mount(mount_dir, "", "", json_options));
    }
    if let Some(ref matches) = matches.subcommand_matches("unmount") {
        let mount_dir = matches.value_of("mount_dir").unwrap();
        process_cmd(do_unmount(mount_dir));
    }
}
