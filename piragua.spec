Name: piragua
Version: 0.1
Release: 18%{?dist}
Summary: Gluster Heketi service that provides directories as volumes.

License: Apache2
URL: https://github.comcast.com/cloud-services/piragua

%define debug_package %{nil}

%{?systemd_requires}
BuildRequires: systemd

Requires: glusterfs-api

%description
Gluster Heketi service that provides directories as volumes.

%prep

%install
rm -rf $RPM_BUILD_ROOT
mkdir $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/usr/sbin $RPM_BUILD_ROOT/etc/piragua $RPM_BUILD_ROOT/lib/systemd/system

cp $RPM_BUILD_DIR/target/release/piragua $RPM_BUILD_ROOT/usr/sbin/piragua
cp $RPM_BUILD_DIR/systemd/piragua.service $RPM_BUILD_ROOT/lib/systemd/system
cp $RPM_BUILD_DIR/systemd/environment $RPM_BUILD_ROOT/etc/piragua/

%files
/usr/sbin/piragua
/lib/systemd/system/piragua.service
%dir /etc/piragua
%config(noreplace) /etc/piragua/environment

%doc

%changelog

%post
%systemd_post piragua.service

%preun
%systemd_preun piragua.service

%postun
%systemd_postun_with_restart piragua.service
