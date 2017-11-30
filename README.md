# gluster_flexvol
kubernetes/openshift glusterfs thin heketi volumes

This repo emulates the [heketi](https://github.com/heketi/heketi/wiki/API) api
to provide directories on a single volume as kubernetes dynamic volumes.

## Requirements:
* glusterfs 3.12.x or newer is required on the kubernetes/openshift servers.  
The 3.12 release introduced deep mount support for fuse.

## Building
* Run `cargo build --release` with a nightly version of rust to build

## Deploying
* Install the deb/rpm package for this on your glusterfs cluster and
enable/start the systemd service.
