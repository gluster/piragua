Name: gluster-flexvol
Version: 0.1
Release: 1%{?dist}
Summary: Gluster Heketi service that provides directories as volumes.

License: Apache2
URL: https://github.comcast.com/cholco202/gluster_flexvol
Source0: https://github.comcast.com/cholco202/gluster_flexvol/archive/%{name}-%{version}.tar.gz

%{?systemd_requires}
BuildRequires: systemd

BuildRequires: docker
Requires: glusterfs-api

%description
Gluster Heketi service that provides directories as volumes.

%prep
%setup -q

%build
./build.sh -d centos -p $RPM_BUILD_DIR/%{name}-%{version}

%install
rm -rf $RPM_BUILD_ROOT
mkdir $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/usr/sbin $RPM_BUILD_ROOT/etc/gluster-flexvol $RPM_BUILD_ROOT/lib/systemd/system

cp $RPM_BUILD_DIR/%{name}-%{version}/target/release/gluster-flexvol-centos $RPM_BUILD_ROOT/usr/sbin/gluster-flexvol
cp $RPM_BUILD_DIR/%{name}-%{version}/systemd/gluster-flexvol.service $RPM_BUILD_ROOT/lib/systemd/system
cp $RPM_BUILD_DIR/%{name}-%{version}/systemd/environment $RPM_BUILD_ROOT/etc/gluster-flexvol/

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
