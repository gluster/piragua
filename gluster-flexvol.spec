Name: gluster-flexvol
Version: 0.1
Release: 1%{?dist}
Summary: Gluster Heketi service that provides directories as volumes.

License: Apache2
URL: https://github.comcast.com/cholco202/gluster_flexvol
Source0: https://github.comcast.com/cholco202/gluster_flexvol/archive/gluster-flexvol-0.1.tar.gz

%{?systemd_requires}
BuildRequires: systemd

BuildRequires: docker
Requires: glusterfs-api

%description
Gluster Heketi service that provides directories as volumes.

%prep
%setup -q

%build
./build.sh -d centos -p $RPM_BUILD_DIR/gluster-flexvol-0.1

%install
rm -rf $RPM_BUILD_ROOT
echo "Install files here"

%files
/usr/sbin/gluster-flexvol
/etc/gluster-flexvol/environment
/lib/systemd/system/gluster-flexvol.service
%dir /etc/gluster-flexvol

%doc

%changelog


%post
%systemd_post gluster-flexvol.service

%preun
%systemd_preun gluster-flexvol.service

%postun
%systemd_postun_with_restart gluster-flexvol.service
