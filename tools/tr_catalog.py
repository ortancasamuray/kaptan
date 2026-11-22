#!/usr/bin/env python3
"""Regenerate lang/tr/LC_MESSAGES/kaptan.po.

Collects @tr("...") strings from ui/, tr("...")/trf("...") strings from src/, plus EXTRA strings that
reach tr() indirectly. Fails if any string has no Turkish translation (add it to TR below).
"""
import re, sys, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent

EXTRA = [
    # sidebar
    "Dashboard", "Startup Apps", "System Cleaner", "Search", "Services", "Processes", "Uninstaller",
    "Resources", "Hosts", "APT Repositories", "Desktop", "Settings",
    # cleaner categories
    "Package Caches", "Crash Reports", "Application Logs", "Application Caches", "Trash",
    # alerts
    "High CPU Usage", "High Memory Usage", "High Disk Usage",
    # page messages
    "Repository enabled.", "Repository disabled.", "Repository removed.", "Repository updated.", "Repository added.",
    "{} enabled.", "{} disabled.", "{} started.", "{} stopped.", "{} moved to trash.", "{} deleted.",
    # core errors
    "Name and command fields are required.", "Select the search directory.",
    "Install zenity or kdialog to browse, or type the directory path.", "No supported package manager found",
    "Repository is empty", "add-apt-repository is not installed (package: software-properties-common)",
    "source entry not found",
    # gsettings
    "Appearance", "Accessibility", "Window Manager",
    "Desktop background mode", "Lock screen background mode", "Show home folder on desktop", "Show trash on desktop",
    "Show mounted volumes on desktop", "Show battery percentage", "Show seconds in clock", "Show weekday in clock",
    "Enable animations", "On-screen keyboard", "Screen reader", "Focus mode", "Raise windows on click",
    "Title bar double click", "Title bar middle click", "Title bar right click", "Dynamic workspaces",
    "Number of workspaces", "Center new windows",
    "None", "Wallpaper", "Centered", "Scaled", "Stretched", "Zoom", "Spanned", "Click", "Sloppy", "Mouse",
    "Maximize", "Maximize Horizontally", "Maximize Vertically", "Minimize", "Lower", "Menu",
]

TR = {
    # --- sidebar / titles
    "Dashboard": "Gösterge Paneli", "Startup Apps": "Başlangıç Uygulamaları", "System Cleaner": "Sistem Temizleyici",
    "Search": "Arama", "Services": "Servisler", "Processes": "İşlemler", "Uninstaller": "Kaldırıcı",
    "Resources": "Kaynaklar", "Hosts": "Hosts", "APT Repositories": "APT Depoları", "Desktop": "Masaüstü",
    "Settings": "Ayarlar",
    # --- dashboard
    "CPU": "İŞLEMCİ", "MEMORY": "BELLEK", "DISK": "DİSK", "GPU": "GPU", "SYSTEM INFO": "SİSTEM BİLGİSİ",
    "DOWNLOAD": "İNDİRME", "UPLOAD": "YÜKLEME", "Total: {}": "Toplam: {}",
    "Hostname: {}": "Bilgisayar adı: {}", "Platform: {}": "Platform: {}", "Distribution: {}": "Dağıtım: {}",
    "Kernel Release: {}": "Çekirdek sürümü: {}", "CPU Model: {}": "İşlemci modeli: {}", "CPU Cores: {}": "İşlemci çekirdeği: {}",
    "CPU Speed: {}": "İşlemci hızı: {}",
    # --- resources
    "CPU History": "İşlemci Geçmişi", "CPU Load Averages": "İşlemci Yük Ortalamaları", "Memory History": "Bellek Geçmişi",
    "GPU History": "GPU Geçmişi", "Network History": "Ağ Geçmişi", "Disk Read / Write": "Disk Okuma / Yazma",
    "{} minute average: {}": "{} dakikalık ortalama: {}", "Memory: {} of {}": "Bellek: {} / {}", "Swap: {} of {}": "Takas: {} / {}",
    "Download: {}/s  Total: {}": "İndirme: {}/s  Toplam: {}", "Upload: {}/s  Total: {}": "Yükleme: {}/s  Toplam: {}",
    "Read: {}/s": "Okuma: {}/s", "Write: {}/s": "Yazma: {}/s", "{} / {}  ({} free)": "{} / {}  ({} boş)",
    "File Systems": "Dosya Sistemleri",
    # --- alerts
    "High CPU Usage": "Yüksek İşlemci Kullanımı", "High Memory Usage": "Yüksek Bellek Kullanımı", "High Disk Usage": "Yüksek Disk Kullanımı",
    "The amount of CPU used is over {}%.": "İşlemci kullanımı %{} değerini aştı.",
    "The amount of memory used is over {}%.": "Bellek kullanımı %{} değerini aştı.",
    "The amount of disk used is over {}%.": "Disk kullanımı %{} değerini aştı.",
    # --- processes
    "Filter by name, user or PID": "Ad, kullanıcı veya PID ile süz", "All users": "Tüm kullanıcılar", "User": "Kullanıcı",
    "Mem": "Bellek", "Resident": "Fiziksel", "Virtual": "Sanal", "State": "Durum", "Nice": "Öncelik", "Started": "Başlangıç",
    "Process": "İşlem", "Refresh every {} s": "{} sn'de bir yenile", "End Process": "İşlemi Sonlandır",
    "End Process {}": "{} İşlemini Sonlandır",
    # --- startup
    "Startup Applications": "Başlangıç Uygulamaları", "Add Startup App": "Uygulama Ekle",
    "No startup applications found.": "Başlangıç uygulaması bulunamadı.",
    "New Startup Application": "Yeni Başlangıç Uygulaması", "Edit Startup Application": "Başlangıç Uygulamasını Düzenle",
    "Name *": "Ad *", "Command *": "Komut *", "Generic name": "Genel ad", "Comment": "Açıklama", "Icon": "Simge",
    "Cancel": "İptal", "Save": "Kaydet", "Name and command fields are required.": "Ad ve komut alanları zorunludur.",
    # --- services
    "System Services": "Sistem Servisleri", "Search services": "Servis ara", "All": "Tümü", "Running": "Çalışıyor",
    "Stopped": "Durdu", "Enabled": "Etkin", "Disabled": "Devre dışı", "Service": "Servis", "Start at boot": "Açılışta başlat",
    "No services found.": "Servis bulunamadı.", "{} enabled.": "{} etkinleştirildi.", "{} disabled.": "{} devre dışı bırakıldı.",
    "{} started.": "{} başlatıldı.", "{} stopped.": "{} durduruldu.",
    # --- uninstaller
    "Packages ({})": "Paketler ({})", "Snap Packages ({})": "Snap Paketleri ({})", "Search packages": "Paket ara",
    "No packages found.": "Paket bulunamadı.", "Uninstall Selected ({})": "Seçilenleri Kaldır ({})",
    "Uninstalling {} package(s)…": "{} paket kaldırılıyor…", "{} package(s) removed.": "{} paket kaldırıldı.",
    "No supported package manager found": "Desteklenen bir paket yöneticisi bulunamadı",
    "Remove {} package(s)?": "{} paket kaldırılsın mı?", "Remove": "Kaldır", "Nothing to remove.": "Kaldırılacak bir şey yok.",
    "{} more package(s) depend on your selection and will be removed too.": "Seçiminize bağlı {} paket daha var, onlar da kaldırılacak.",
    # --- cleaner
    "Package Caches": "Paket Önbellekleri", "Crash Reports": "Çökme Raporları", "Application Logs": "Uygulama Günlükleri",
    "Application Caches": "Uygulama Önbellekleri", "Trash": "Çöp Kutusu", "Back to categories": "Kategorilere dön",
    "Select what to scan": "Taranacakları seçin", "Scanning…": "Taranıyor…", "System Scan": "Sistemi Tara",
    "Select all": "Tümünü seç", "Select none": "Hiçbirini seçme", "Clean": "Temizle",
    "Select at least one category.": "En az bir kategori seçin.", "Nothing selected.": "Hiçbir şey seçilmedi.",
    "{} cleaned.": "{} temizlendi.", "Total {} · selected {}": "Toplam {} · seçilen {}",
    # --- search
    "Directory": "Dizin", "Browse": "Gözat", "File name or pattern, e.g. *.log": "Dosya adı veya desen, ör. *.log",
    "Case insensitive": "Büyük/küçük harf duyarsız", "Regular expression": "Düzenli ifade", "Invert match": "Eşleşmeyi ters çevir",
    "Search as root": "Root olarak ara", "Advanced ▲": "Gelişmiş ▲", "Advanced ▼": "Gelişmiş ▼", "Type": "Tür",
    "File": "Dosya", "Symbolic Link": "Sembolik Bağlantı", "Time": "Zaman", "Any": "Herhangi", "Accessed": "Erişilme",
    "Modified": "Değiştirilme", "Changed": "Durum değişikliği", "Comparison": "Karşılaştırma", "Within (<)": "İçinde (<)",
    "Exactly (=)": "Tam (=)", "Older (>)": "Daha eski (>)", "Minutes": "Dakika", "Size": "Boyut", "Smaller (<)": "Küçük (<)",
    "Equal (=)": "Eşit (=)", "Greater (>)": "Büyük (>)", "Amount": "Miktar", "Unit": "Birim", "Owner": "Sahip", "Group": "Grup",
    "Permissions": "İzinler", "Other": "Diğer", "Empty only": "Yalnızca boş", "Name": "Ad", "Path": "Yol",
    "Open Folder": "Klasörü Aç", "Move to Trash": "Çöpe Taşı", "Delete Permanently": "Kalıcı Olarak Sil",
    "Delete the selected item permanently? This cannot be undone.": "Seçilen öğe kalıcı olarak silinsin mi? Bu işlem geri alınamaz.",
    "Delete": "Sil", "{} found": "{} sonuç bulundu", "{} found, showing {}": "{} sonuç bulundu, {} tanesi gösteriliyor",
    "{} moved to trash.": "{} çöpe taşındı.", "{} deleted.": "{} silindi.",
    "Select the search directory.": "Aranacak dizini seçin.",
    "Install zenity or kdialog to browse, or type the directory path.": "Gözatmak için zenity veya kdialog kurun ya da dizin yolunu yazın.",
    # --- hosts
    "New Host": "Yeni Kayıt", "IP address *": "IP adresi *", "Hostname *": "Bilgisayar adı *", "Aliases": "Takma adlar",
    "Apply": "Uygula", "IP Address": "IP Adresi", "Hostname": "Bilgisayar adı", "Edit": "Düzenle",
    "Unsaved changes": "Kaydedilmemiş değişiklikler", "Save Changes": "Değişiklikleri Kaydet",
    "The IP address and hostname fields are required.": "IP adresi ve bilgisayar adı alanları zorunludur.",
    "'{}' is not a valid IP address.": "'{}' geçerli bir IP adresi değil.", "/etc/hosts saved.": "/etc/hosts kaydedildi.",
    # --- apt
    "e.g. ppa:user/repo  or  deb http://example.com/debian stable main": "ör. ppa:kullanici/depo  veya  deb http://example.com/debian stable main",
    "Source code": "Kaynak kod", "Add Repository": "Depo Ekle", "No repositories found.": "Depo bulunamadı.",
    "Edit Repository": "Depoyu Düzenle", "Types": "Türler", "URI": "URI", "Suites": "Sürümler", "Components": "Bileşenler",
    "Types, URI and suites are required.": "Tür, URI ve sürüm alanları zorunludur.",
    "Repository enabled.": "Depo etkinleştirildi.", "Repository disabled.": "Depo devre dışı bırakıldı.",
    "Repository removed.": "Depo kaldırıldı.", "Repository updated.": "Depo güncellendi.", "Repository added.": "Depo eklendi.",
    "Repository is empty": "Depo alanı boş",
    "add-apt-repository is not installed (package: software-properties-common)": "add-apt-repository kurulu değil (paket: software-properties-common)",
    "source entry not found": "depo kaydı bulunamadı",
    # --- desktop
    "Desktop Settings": "Masaüstü Ayarları",
    "No supported GNOME settings were found on this system.": "Bu sistemde desteklenen GNOME ayarı bulunamadı.",
    "Appearance": "Görünüm", "Accessibility": "Erişilebilirlik", "Window Manager": "Pencere Yöneticisi",
    "Desktop background mode": "Masaüstü arka plan modu", "Lock screen background mode": "Kilit ekranı arka plan modu",
    "Show home folder on desktop": "Ev klasörünü masaüstünde göster", "Show trash on desktop": "Çöp kutusunu masaüstünde göster",
    "Show mounted volumes on desktop": "Bağlı birimleri masaüstünde göster", "Show battery percentage": "Pil yüzdesini göster",
    "Show seconds in clock": "Saatte saniyeyi göster", "Show weekday in clock": "Saatte haftanın gününü göster",
    "Enable animations": "Animasyonları etkinleştir", "On-screen keyboard": "Ekran klavyesi", "Screen reader": "Ekran okuyucu",
    "Focus mode": "Odak modu", "Raise windows on click": "Tıklanınca pencereyi öne getir",
    "Title bar double click": "Başlık çubuğuna çift tıklama", "Title bar middle click": "Başlık çubuğuna orta tıklama",
    "Title bar right click": "Başlık çubuğuna sağ tıklama", "Dynamic workspaces": "Dinamik çalışma alanları",
    "Number of workspaces": "Çalışma alanı sayısı", "Center new windows": "Yeni pencereleri ortala",
    "None": "Yok", "Wallpaper": "Duvar kâğıdı", "Centered": "Ortalanmış", "Scaled": "Ölçeklenmiş", "Stretched": "Uzatılmış",
    "Zoom": "Yakınlaştır", "Spanned": "Yayılmış", "Click": "Tıklama", "Sloppy": "Gevşek", "Mouse": "Fare",
    "Maximize": "Büyüt", "Maximize Horizontally": "Yatay büyüt", "Maximize Vertically": "Dikey büyüt", "Minimize": "Simge durumuna küçült",
    "Lower": "Arkaya gönder", "Menu": "Menü",
    # --- settings
    "GENERAL": "GENEL", "Language": "Dil", "System default": "Sistem varsayılanı", "Theme": "Tema", "Dark": "Koyu", "Light": "Açık",
    "Start page": "Açılış sayfası", "Dashboard disk": "Gösterge paneli diski",
    "File system shown on the dashboard disk gauge": "Gösterge panelindeki disk göstergesinde gösterilen dosya sistemi",
    "Start Kaptan at login": "Oturum açılınca Kaptan'ı başlat", "ALERTS": "UYARILAR", "CPU usage alert (%)": "İşlemci kullanım uyarısı (%)",
    "Memory usage alert (%)": "Bellek kullanım uyarısı (%)", "Disk usage alert (%)": "Disk kullanım uyarısı (%)",
    "0 disables the alert": "0 uyarıyı kapatır", "ABOUT": "HAKKINDA",
    "Linux system optimizer and monitor — a Rust port of Stacer. GPL-3.0": "Linux sistem iyileştirici ve izleyici — Stacer'ın Rust sürümü. GPL-3.0",
}

def collect():
    ui, rs = set(), set()
    lit = r'"((?:[^"\\]|\\.)*)"'
    for f in (ROOT / "ui").rglob("*.slint"):
        ui |= set(re.findall(r'@tr\(' + lit, f.read_text()))
    for f in (ROOT / "src").rglob("*.rs"):
        rs |= set(re.findall(r'\btrf?\(' + lit, f.read_text()))
    return sorted(ui | rs | set(EXTRA), key=str.lower)

def esc(s):
    return s.replace("\\", "\\\\").replace('"', '\\"')

def main():
    ids = [i.replace('\\"', '"') for i in collect()]
    missing = [i for i in ids if i not in TR]
    if missing:
        sys.exit("Missing Turkish translations:\n  " + "\n  ".join(missing))
    unused = sorted(set(TR) - set(ids))
    out = ['msgid ""', 'msgstr ""', '"Project-Id-Version: kaptan\\n"', '"Language: tr\\n"',
           '"MIME-Version: 1.0\\n"', '"Content-Type: text/plain; charset=UTF-8\\n"',
           '"Content-Transfer-Encoding: 8bit\\n"', '"Plural-Forms: nplurals=1; plural=0;\\n"', '']
    for i in ids:
        out += [f'msgid "{esc(i)}"', f'msgstr "{esc(TR[i])}"', '']
    path = ROOT / "lang/tr/LC_MESSAGES/kaptan.po"
    path.write_text("\n".join(out))
    print(f"{path.relative_to(ROOT)}: {len(ids)} strings" + (f" ({len(unused)} unused in TR: {unused})" if unused else ""))

main()
