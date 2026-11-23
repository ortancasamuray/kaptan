# Kaptan

[English](README.md) · **Türkçe**

Rust ile yazılmış, arayüzü [Slint](https://slint.dev) olan Linux sistem iyileştirici ve izleyici.
Kaptan, [Stacer](https://github.com/QuentiumYT/Stacer) (C++/Qt) uygulamasının Rust sürümüdür.

Web sitesi: [www.anadolupanteri.org.tr](https://www.anadolupanteri.org.tr) ·
İndir: [GitHub sürümleri](https://github.com/ortancasamuray/kaptan/releases)

## Özellikler

| Sayfa | Ne yapar |
|---|---|
| Gösterge Paneli | İşlemci, bellek, disk ve GPU göstergeleri, ağ hızı, fan devirleri, sistem bilgisi, işlemci ve GPU sıcaklık göstergeleri (soğuk / sıcak / çok sıcak bölgeleri) |
| Kaynaklar | 60 saniyelik geçmiş grafikleri: çekirdek başına işlemci, yük ortalamaları, bellek/takas, GPU/VRAM, ağ, disk G/Ç; dosya sistemi kullanımı |
| İşlemler | Sıralanabilir, süzülebilir işlem listesi; işlem sonlandırma (gerekirse `pkexec` ile) |
| Başlangıç Uygulamaları | `~/.config/autostart` girdilerini etkinleştirme, devre dışı bırakma, ekleme, düzenleme ve silme |
| Servisler | systemd servislerini açılışta etkinleştirme/devre dışı bırakma, başlatma/durdurma (`pkexec`) |
| Kaldırıcı | apt/dnf/yum/zypper/pacman ve snap paketlerini kaldırma |
| Sistem Temizleyici | Paket önbellekleri, çökme raporları, günlükler, uygulama önbellekleri ve çöp kutusunu tarayıp temizleme |
| Arama | `find` tabanlı dosya arama: ad, tür, zaman, boyut, izin ve sahip süzgeçleri |
| Hosts | `/etc/hosts` dosyasını düzenleme |
| APT Depoları | Depoları açma/kapama, düzenleme, kaldırma (`.list` ve deb822 `.sources`) ve ekleme |
| Masaüstü | `gsettings` ile GNOME ayarları (yalnızca GNOME oturumlarında görünür) |
| Ayarlar | Tema (koyu/açık), açılış sayfası, gösterge paneli diski, kullanım uyarıları, oturum açılınca başlatma |

Stacer gibi Kaptan'ın da sistem tepsisi simgesi vardır (StatusNotifierItem: KDE, XFCE 4.16+,
AppIndicator eklentili GNOME). Menüsünden herhangi bir sayfa açılır ya da uygulamadan çıkılır;
pencere kapatılınca tepside çalışmaya devam edilip edilmeyeceği sorulur (Ayarlar → Pencere
kapatıldığında). Oturum açılınca başlatıldığında (Ayarlar → Oturum açılınca Kaptan'ı başlat) pencere
açılır; `kaptan --hide` tepside gizli başlatır. Tepsi yoksa pencereyi kapatmak uygulamadan çıkar.

GPU kullanımı AMD'de sysfs'ten, NVIDIA'da `nvidia-smi` ile okunur. Hibrit dizüstülerde uyuyan NVIDIA
GPU uyandırılmaz. Intel tümleşik GPU'lar kullanım sayacı sunmadığı için gösterilmez.

Sıcaklıklar hwmon'dan (`coretemp`, `k10temp`, `zenpower`, ARM `cpu_thermal`, yedek olarak thermal
zone'lar), GPU için `nvidia-smi` ya da kartın hwmon algılayıcısından (amdgpu, nouveau, xe) okunur.
Bölge sınırları `src/core/temps.rs` içindedir: işlemci 60 / 85 °C, GPU 60 / 80 °C.

## Kurulum (hazır paketler)

[Sürümler](https://github.com/ortancasamuray/kaptan/releases) sayfasından dağıtımınıza uygun paketi
indirip kurun:

```sh
sudo apt install ./kaptan_<sürüm>-1_amd64.deb          # Debian, Ubuntu, Pardus, Mint
sudo dnf install ./kaptan-<sürüm>-1.x86_64.rpm         # Fedora, openSUSE
sudo pacman -U kaptan-<sürüm>-1-x86_64.pkg.tar.zst     # Arch
```

Hazır paketler glibc 2.39 veya üzerini ister (Ubuntu 24.04+, Fedora 40+, güncel Arch). Daha eski
sistemlerde kaynaktan derleyin.

## Derleme

```sh
cargo build --release
./target/release/kaptan
```

Varsa kullanılan araçlar: `pkexec` (yetki gerektiren işlemler), `nvidia-smi`, `notify-send`
(uyarılar), `zenity` veya `kdialog` (klasör seçici), `gio` (çöpe taşıma), `add-apt-repository`,
`gsettings`.

## Testler

```sh
cargo test                                                   # birim testleri
cargo test root_ -- --ignored --test-threads=1 --nocapture   # yetkili işlemler (pkexec parola sorar)
```

`root_` testleri (`src/root_tests.rs`) pkexec ile `/etc/hosts`'u, bir APT kaynağını ve `/tmp`
içindeki bir dosyayı değiştirir, ardından hepsinin bayt bayt eski hâline döndüğünü denetler.

## Debian paketi

```sh
cargo install cargo-deb   # bir kez
cargo deb                 # -> target/debian/kaptan_<sürüm>-1_amd64.deb
sudo apt install ./target/debian/kaptan_*_amd64.deb
```

Paket bilgileri `Cargo.toml` içindeki `[package.metadata.deb]` bölümündedir; `debian/` klasöründe
değişiklik günlüğü, telif bilgisi ve kılavuz sayfası bulunur. Sürümü hem `Cargo.toml` hem
`debian/changelog` içinde yükseltin.

## Debian, Fedora ve Arch paketleri

```sh
packaging/build-packages.sh            # hepsi, dist/ klasörüne
packaging/build-packages.sh deb rpm    # ya da bir kısmı: deb rpm arch src
```

| Çıktı | Nasıl üretilir |
|---|---|
| `kaptan_<sürüm>-1_amd64.deb` | `cargo deb` |
| `kaptan-<sürüm>-1.x86_64.rpm` | `packaging/rpm/kaptan.spec` ile `rpmbuild` (derlenmiş ikiliyi paketler) |
| `kaptan-<sürüm>-1-x86_64.pkg.tar.zst` | `packaging/arch/mkpkg.py` (`makepkg` gerekmez; `packaging/arch/PKGBUILD` dosyasını okur) |
| `kaptan-<sürüm>.tar.gz` | `rpmbuild -ba kaptan.spec` veya `makepkg -si` için kaynak arşivi |

Spec ya da PKGBUILD sürümü `Cargo.toml` ile tutmazsa betik durur. Debian 13'te derlenen ikililer
glibc ≥ 2.39 ister; eski sistemler için spec ya da PKGBUILD ile kaynaktan derleyin.
`docs/tanitim/` ekran görüntülü kısa bir Türkçe tanıtım sayfasıdır.

## Kurulum (sistem geneli, kaynaktan)

```sh
cargo build --release
sudo install -Dm755 target/release/kaptan /usr/local/bin/kaptan
sudo install -Dm644 data/kaptan.desktop /usr/local/share/applications/kaptan.desktop
sudo install -Dm644 data/kaptan.svg /usr/local/share/icons/hicolor/scalable/apps/kaptan.svg
```

Kaldırma: `sudo rm /usr/local/bin/kaptan /usr/local/share/applications/kaptan.desktop /usr/local/share/icons/hicolor/scalable/apps/kaptan.svg`

## Kurulum (yalnızca kullanıcı, kaynaktan)

```sh
cargo build --release
install -Dm755 target/release/kaptan ~/.local/bin/kaptan
install -Dm644 data/kaptan.svg ~/.local/share/icons/hicolor/scalable/apps/kaptan.svg
# Exec tam yol: menülerin PATH'inde ~/.local/bin her zaman olmaz
sed "s|^Exec=kaptan$|Exec=$HOME/.local/bin/kaptan|" data/kaptan.desktop > ~/.local/share/applications/kaptan.desktop
update-desktop-database ~/.local/share/applications
```

Kaldırma:

```sh
rm ~/.local/bin/kaptan ~/.local/share/applications/kaptan.desktop \
   ~/.local/share/icons/hicolor/scalable/apps/kaptan.svg
rm -r ~/.config/kaptan   # ayarlar, isteğe bağlı
```

## Klasör yapısı

```
src/core/    platform mantığı, arayüz yok (stacer-core'dan aktarıldı)
src/pages/   her sayfa için bir modül: çekirdeği arayüze bağlar
src/i18n.rs  Rust içinde oluşturulan metinlerin çevirileri
lang/        gettext katalogları
tools/       çeviri kaynakları ve katalog üreteci
ui/          Slint arayüzü: tema, bileşenler, globals (veri sözleşmesi), sayfalar, simgeler
```

Ayarlar `~/.config/kaptan/settings.json` dosyasında saklanır.

## Stacer'dan farkları

- Uyarılar tepsi balonları yerine masaüstü bildirimleriyle (`notify-send`) gösterilir.
- Stacer'ın Unity/Compiz ayarlarının yerine güncel GNOME ayarları geldi.
- GitHub'a karşı güncelleme denetimi yok.

## Çeviriler

Kaptan İngilizce ve Stacer'la aynı 26 dille gelir: Almanca, Arapça, Baskça, Çekçe, Çince
(Basitleştirilmiş ve Geleneksel), Felemenkçe, Fransızca, Galiçyaca, Hintçe, İspanyolca, İsveççe,
İtalyanca, Kannada, Katalanca, Korece, Lehçe, Macarca, Malayalam, Oksitanca, Portekizce, Romence, Rusça,
Türkçe, Ukraynaca ve Vietnamca. Dil sistem yerel ayarını izler ve Ayarlar → Dil
bölümünden anında değiştirilebilir.

Stacer'da aynı metin varsa onun çevirmenlerinin emeği kullanıldı (dile göre metinlerin %13–39'u).
Gerisi Kaptan için çevrildi ve henüz **ana dili konuşanlarca gözden geçirilmedi** — düzeltmeler
`tools/translations/<kod>.json` dosyalarına memnuniyetle kabul edilir.

Kaynaklar: `tools/translations/<kod>.json` (`{"English": "çeviri"}`), bunlar
`lang/<kod>/LC_MESSAGES/kaptan.po` dosyalarına derlenir (Slint arayüzüne gömülür, `@tr(...)`; Rust'ta
oluşturulan metinler için `src/i18n.rs` tarafından okunur). Metin ekleyip değiştirdikten sonra:

```sh
python3 tools/i18n.py   # tüm katalogları ve src/i18n_languages.rs dosyasını yeniden üretir
```

Türkçe eksiksiz olmalıdır (yoksa araç hata verir); diğer dillerde eksik metinler İngilizce görünür.
Yeni dil eklemek için `tools/translations/<kod>.json` oluşturup kodu `tools/i18n.py` içindeki
`LANGUAGES` listesine ekleyin. `tools/stacer_ts.py` Stacer'ın Qt `.ts` kataloglarını okur.

Korece, Çince ve Hint yazıları için bunları kapsayan bir yazı tipi gerekir (ör. Korece için
`fonts-noto-cjk` veya `fonts-nanum`); yoksa karakterler kutu olarak görünür.

## Lisans

GPL-3.0-or-later, Stacer ile aynı.
