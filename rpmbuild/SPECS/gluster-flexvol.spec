Name: gluster-flexvol
Version: 1.0
Release: 1%{?dist}
Summary: Gluster Heketi service that provides directories as volumes.

License: Apache2
URL: https://github.comcast.com/cholco202/gluster_flexvol
Source0: https://github.comcast.com/cholco202/gluster_flexvol/archive/0.1.1.tar.gz

%{?systemd_requires}
BuildRequires: systemd

BuildRequires: gcc
BuildRequires: centos-release-gluster
BuildRequires: glusterfs-api
BuildRequires: openssl-devel
Requires: glusterfs-api

%description
Gluster Heketi service that provides directories as volumes.

%prep
%setup -q

%build
%configure
make %{?_smp_mflags}

%install
rm -rf $RPM_BUILD_ROOT
%make_install


%files
%dir /etc/gluster-flexvol
     /etc/gluster-flexvol/environment
/lib/systemd/system/gluster-flexvol.service

%doc

%changelog


%post
%systemd_post gluster-flexvol.service

%preun
%systemd_preun gluster-flexvol.service

%postun
%systemd_postun_with_restart gluster-flexvol.service
