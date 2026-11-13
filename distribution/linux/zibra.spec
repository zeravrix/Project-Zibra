Name:           zibra
Version:        2.2.0
Release:        1%{?dist}
Summary:        ZIBRA — 100% Native Rust Manga & Webtoon Reader for PC
License:        GPL-3.0-only
URL:            https://zeravrix.pages.dev

Requires:       gtk3
Requires:       openssl-libs

%description
ZIBRA (ZERAVRIX Rust Edition) is a 100% native Rust desktop manga, manhwa, and webtoon reader powered by eframe/egui, Tokio, and SQLite with 1,647+ integrated sources.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/512x512/apps

install -m 755 %{_sourcedir}/zibra %{buildroot}/usr/bin/zibra
install -m 644 %{_sourcedir}/zibra.desktop %{buildroot}/usr/share/applications/zibra.desktop
install -m 644 %{_sourcedir}/logo.png %{buildroot}/usr/share/icons/hicolor/512x512/apps/zibra.png

%files
/usr/bin/zibra
/usr/share/applications/zibra.desktop
/usr/share/icons/hicolor/512x512/apps/zibra.png
