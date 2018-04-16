# Piragua
[![Build Status](https://travis-ci.org/gluster/piragua.svg?branch=master)](https://travis-ci.org/gluster/piragua)
kubernetes/openshift glusterfs thin heketi volumes

This repo emulates the [heketi](https://github.com/heketi/heketi/wiki/API) api
to provide directories on a single volume as kubernetes dynamic volumes.

## Requirements:
* glusterfs 3.12.x or newer is required on the kubernetes/openshift servers.  
The 3.12 release introduced deep mount support for fuse.

## Building
* Run `cargo build --release` with a nightly version of rust to build
* Alternatively you can run build.sh and build for a different OS version
* DEB's and RPM's will be published with each release

## Deploying
* Install the deb/rpm package for this on all of the glusterfs servers 
* Set the correct environment variables in the 
`/etc/piragua/environment` file.
* enable/start the systemd service.

Big thanks to Miranda Shutt and David Hocky for helping me debug this
with openshift and kubernetes!  

## High Level Overview
```
+-----------+  Create Volume +-------------+ Create Volume  +---------+
| Openshift +----------------+ Rust Heketi | ---------------+ Gluster |
+-----------+                +-------------+                +---------+
```
Clients using Openshift's container service occasionaly need persistent storage.  
There's 3 components interacting in this.  There's the Openshift server, the Heketi service and Gluster.  
- [Openshift](https://www.openshift.com/) is a container management platform that leverages [Kubernetes](https://kubernetes.io/).  It is a containers as a service platform.  
- [Gluster](http://docs.gluster.org/en/latest/) is an open source distributed filesystem.
- [Heketi](https://github.com/heketi/heketi) is a service that Redhat created to manage Gluster through a [rest](https://github.com/heketi/heketi/blob/master/doc/api/api.md) api.  

The normal workflow is that Openshift requests a volume to be created.  It makes an api call to 
Heketi and then Heketi turns around and requests that Gluster create a new volume.  The problem here is that 
managing many Gluster volumes that are colocated on a single cluster can get very difficult.  What
I have created here is a web server that recreates a minimal portion of the Heketi api without using any Heketi code to trick Openshift into thinking it's talking to the real Heketi api.  When a volume create call comes through this 
new web server doesn't ask Gluster to create a volume.  It instead asks Gluster to make a new top level directory on the cluster, adds a quota to it and returns that as the volume name.  This now means that thousands of Openshift volumes can be colocated on the same Gluster easily.  The Openshift service then uses Gluster's fuse module to deep mount the directory and
nobody is the wiser from the client perspective.  

On the Gluster server side you'll see something like this:
```
tree /mnt/glusterfs/ab1d8755-907d-44c3-9b32-0de2750c8e75/
/mnt/glusterfs/ab1d8755-907d-44c3-9b32-0de2750c8e75/
└── vol_ab1d8755-907d-44c3-9b32-0de2750c8e75
```
That is an openshift volume.  It also has a quota attached to it:
```
 gluster vol quota gv0 list
                  Path                   Hard-limit  Soft-limit      Used  Available  Soft-limit exceeded? Hard-limit exceeded?
-------------------------------------------------------------------------------------------------------------------------------
/a08abef9-e4d2-499c-8b32-1b01ff855705      1.0GB     80%(819.2MB)  122.1MB 901.9MB              No  
```


## Licensing
Please take note that this repository is dual licensed under
Apache2 and GPLv2
