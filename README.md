# gluster_flexvol
kubernetes/openshift glusterfs thin heketi volumes

This repo emulates the [heketi](https://github.com/heketi/heketi/wiki/API) api
to provide directories on a single volume as kubernetes dynamic volumes.

## Requirements:
* glusterfs 3.12.x or newer is required on the kubernetes/openshift servers.  
The 3.12 release introduced deep mount support for fuse.

## Building
* Run `cargo build --release` with a nightly version of rust to build
* Alternatively you can run build.sh and build for a different OS version
* Mortar is also automatically building on every commit and producing 
an rpm file which is then published to Atlas.

## Deploying
* Install the deb/rpm package for this on all of the glusterfs servers 
* Set the correct environment variables in the 
`/etc/gluster-flexvol/environment` file.
* enable/start the systemd service.

Big thanks to Miranda Shutt and David Hocky for helping me debug this
with openshift and kubernetes!  
