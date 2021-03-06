#!/bin/bash

distro=""
path=""
DEBUG="0"

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

if [ ! -d "${path}" ]
then
   echo "path ${path} does not exist or is not a directory.  Exiting"
   exit -1
fi
 
set -euo pipefail

echo "About to launch ${distro} container"
container="gluster-piragua-$RANDOM"

function finish {
   echo "Cleaning up: ($?)!"
   docker rm -f ${container}
}

if [ "x{$DEBUG}" != "x1" ]
then
   trap finish EXIT
fi



case "$distro" in 
   centos*)
      echo "Launching ${container} with args -d -i -t -v ${path}:/build/z -w /build ${distro}"
      docker run --name ${container} -d -i -t -v ${path}:/build:z -w /build ${distro}
      echo "Installing deps"
      docker exec ${container} yum update -y
      echo "installing centos-release-gluster"
      docker exec ${container} yum install -y centos-release-gluster openssl-devel.x86_64 rpm-build
      echo "installing gfapi"
      packages="glusterfs-api-devel glusterfs-api gcc"
      docker exec ${container} yum install -y ${packages}
      ;;
   ubuntu*)
      echo "Launching ${container} with args -d -i -t -v ${path}:/build/z -w /build ${distro}"
      docker run --name ${container} -d -i -t -v ${path}:/build:z -w /build ${distro}
      echo "Installing deps"
      docker exec ${container} apt update
      echo "installing gluster"
      docker exec ${container} add-apt-repository ppa:gluster/glusterfs-3.12
      docker exec ${container} apt update
      echo "installing gfapi"
      packages="glusterfs gcc"
      docker exec ${container} yum install -y ${packages}
      ;;
   *)
      echo "Do not know how to build with distro ${distro}, exiting"
      exit -1
      ;;
esac

echo "About to install rust"
docker exec ${container} curl https://sh.rustup.rs -o /root/rustup.sh 
docker exec ${container} chmod +x /root/rustup.sh 
echo "Installing nightly rust"
docker exec ${container} /root/rustup.sh --default-toolchain nightly -y

echo "Building"
docker exec ${container} /root/.cargo/bin/cargo build --release --all
case "$distro" in 
   centos*)
	echo "Building rpm"
	docker exec ${container} rpmbuild --define "_builddir /build" -bb piragua.spec
	rpm_file=$(docker exec ${container} ls /root/rpmbuild/RPMS/x86_64/)
	docker cp ${container}:/root/rpmbuild/RPMS/x86_64/${rpm_file} .
	;;
esac
docker exec ${container} mv target/release/piragua target/release/piragua-${distro}

echo "Release directory"
ls ${path}/target/release/

exit 0
