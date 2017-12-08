#!/bin/bash

distro=""
path=""

while getopts ":d:p:" opt; do
  case $opt in
    d)
      distro=$OPTARG
      ;;
    p)
      path=$OPTARG
      ;;
    \?)
      echo "Usage $0 -d (centos|ubuntu) -p /path/to/build"
      exit -1
      ;;
    : )
      echo "Usage $0 -d (centos|ubuntu) -p /path/to/build"
      exit -1
      ;;
  esac
done
shift $((OPTIND - 1))

if [ "x${distro}" == "x" ]
then
  echo "Usage $0 -d (centos|ubuntu) -p /path/to/build"
  exit -1
fi

if [ "x${path}" == "x" ]
then
  echo "Usage $0 -d (centos|ubuntu) -p /path/to/build"
  exit -1
fi
 
  
set -euo pipefail

echo "About to launch ${distro} container"
container="gluster-flexvol-build-$RANDOM"

#function finish {
#    echo "Cleaning up: ($?)!"
#    docker kill $container
#    docker rm $container
#    echo "finished cleaning up"
#}
#trap finish EXIT

echo "Named container: $container"
docker run --name $container -d -i -t -v $path:/build:z -w /build $distro
echo "Launched $container"

docker exec $container ls /build/

echo "Installing deps"
if [[ "$distro" == centos* ]]
    then
	docker exec $container yum update -y
	echo "installing centos-release-gluster"
	docker exec $container yum install -y centos-release-gluster openssl-devel.x86_64
	echo "installing gfapi"
    packages="glusterfs-api-devel glusterfs-api gcc"
	docker exec $container yum install -y $packages
fi

if [[ "$distro" == ubuntu* ]]
    then
	docker exec $container apt update
	echo "installing gluster"
    docker exec $container add-apt-repository ppa:gluster/glusterfs-3.12
	docker exec $container apt update
	echo "installing gfapi"
    packages="glusterfs gcc"
	docker exec $container yum install -y $packages
fi

echo "About to install rust"
docker exec $container curl https://sh.rustup.rs -o /root/rustup.sh 
echo "chmod"
docker exec $container chmod +x /root/rustup.sh 
echo "installing nightly rust"
docker exec $container /root/rustup.sh --default-toolchain nightly -y

echo "Building"
docker exec $container /root/.cargo/bin/cargo build --release --all

echo "Release directory"
ls $path/target/release/
docker exec $container mv target/release/gluster-flexvol target/release/gluster-flexvol-$distro

