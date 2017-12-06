if [ -z "$1" ]
  then
    echo "No argument supplied, requires build version"
    exit 1
fi

set -euo pipefail

distro=$1
path=`pwd`
echo "About to launch $distro container"
container="gluster-flexvol-build-$RANDOM"
mkdir -p /tmp/$container

function finish {
    echo "Cleaning up: ($?)!"
    lxc delete -f $container
    rm -rf /tmp/$container
    echo "finished cleaning up"
}
trap finish EXIT

echo "Named container: $container"
if [ "$distro" = "centos7" ]; then
	lxc launch images:centos/7/amd64 --ephemeral $container > /dev/null
else
	lxc launch ubuntu:$distro --ephemeral $container > /dev/null
fi
echo "Launched $container"

echo "Setting up /build"

lxc exec $container -- /bin/sh -c "/bin/mkdir -p /build"
echo "Pushing files into container"
tar --exclude-vcs --exclude=target -zcf - . | lxc exec --verbose $container -- /bin/sh -c "/bin/tar zxf - -C /build"
lxc exec --verbose $container -- /bin/sh -c "ls /build/"
sleep 5

echo "Installing deps"
packages="libssl-dev protobuf-compiler libprotobuf-dev libsodium-dev liblzma-dev pkg-config"
if [ "$distro" = "centos7" ]
    then
	lxc exec --verbose $container -- /bin/sh -c "yum update"
	echo "installing centos-release-gluster"
	lxc exec --verbose $container -- /bin/sh -c "yum install -y centos-release-gluster"
	echo "installing gfapi"
    packages="glusterfs-api-devel glusterfs-api gcc"
	lxc exec --verbose $container -- /bin/sh -c "yum install -y $packages"
fi

echo "About to install rust"
lxc exec --verbose $container -- /bin/sh -c "curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y"

echo "Building"
lxc exec --verbose $container -- /bin/sh -c "cd /build; /root/.cargo/bin/rustup override set nightly"
lxc exec --verbose $container -- /bin/sh -c "cd /build; /root/.cargo/bin/cargo build --release --all"

echo "Release directory"
lxc exec --verbose $container -- /bin/sh -c "ls /build/target/release/"

echo "Pulling build"
lxc file pull --verbose -r $container/build/target/release/ /tmp/$container

echo "/tmp/$container/"
ls /tmp/$container/

echo "rename"
cd /tmp/$container; find release "gluster-flexvol" -exec rename "s/gluster-flexvol/$(echo $distro)_gluster-flexvol/g" {}  \;
cp /tmp/$container/release/*_gluster-flexvol* $path

finish
