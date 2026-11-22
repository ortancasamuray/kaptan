//! Manual tests for privileged actions. Each one opens pkexec password prompts and restores the
//! system to its previous state. Run with:
//!   cargo test root_ -- --ignored --test-threads=1 --nocapture
//! root_package_remove_then_reinstall really uninstalls a package (ncdu by default) and reinstalls it.

use crate::core::{apt_source, cleaner, hosts, process};
use std::path::Path;

#[test]
#[ignore]
fn root_hosts_add_then_remove() {
    let original = std::fs::read(hosts::HOSTS_PATH).unwrap();
    let mut lines = hosts::load();
    lines.push(hosts::format_line("127.0.0.1", "kaptan-test.local", ""));
    println!("hosts: adding test entry (password prompt 1/2)");
    hosts::save(&lines).expect("save with test entry");
    assert!(hosts::items(&hosts::load()).iter().any(|i| i.hostname == "kaptan-test.local"));

    lines.pop();
    println!("hosts: removing test entry (password prompt 2/2)");
    hosts::save(&lines).expect("restore");
    assert_eq!(std::fs::read(hosts::HOSTS_PATH).unwrap(), original, "/etc/hosts not restored byte for byte");
}

#[test]
#[ignore]
fn root_apt_disable_then_enable() {
    let src = apt_source::list().into_iter().find(|s| s.active && !s.is_deb822()).expect("an active .list source");
    let original = std::fs::read(&src.file).unwrap();
    println!("apt: disabling {} (password prompt 1/2)", src.display());
    apt_source::set_active(&src, false).expect("disable");
    let now = apt_source::list().into_iter().find(|s| s.file == src.file && s.index == src.index).unwrap();
    assert!(!now.active);

    println!("apt: enabling it again (password prompt 2/2)");
    apt_source::set_active(&now, true).expect("enable");
    assert_eq!(std::fs::read(&src.file).unwrap(), original, "{} not restored byte for byte", src.file.display());
}

#[test]
#[ignore]
fn root_cleaner_falls_back_to_pkexec() {
    let path = Path::new("/tmp/kaptan-root-owned-test");
    println!("cleaner: creating a root-owned file in /tmp (password prompt 1/2)");
    crate::core::command::sudo_exec("touch", &[path.to_str().unwrap()], None).expect("create root file");
    assert!(path.exists());
    println!("cleaner: removing it, plain delete fails so pkexec is used (password prompt 2/2)");
    cleaner::clean(&[path.to_path_buf()]).expect("clean");
    assert!(!path.exists());
}

#[test]
#[ignore]
fn root_terminate_own_process() {
    let mut child = std::process::Command::new("sleep").arg("300").spawn().unwrap();
    process::terminate(child.id() as i32).expect("terminate");
    let status = child.wait().unwrap();
    assert!(!status.success(), "sleep should have been killed");
}

/// D-Bus activated helper that is normally idle: safe to start/stop and to disable/enable briefly.
const TEST_SERVICE: &str = "blueman-mechanism";

#[test]
#[ignore]
fn root_service_start_then_stop() {
    use crate::core::service;
    let was_active = service::is_active(TEST_SERVICE);
    println!("services: starting {TEST_SERVICE} (password prompt 1/2)");
    service::set_running(TEST_SERVICE, true).expect("start");
    assert!(service::is_active(TEST_SERVICE));
    println!("services: stopping {TEST_SERVICE} (password prompt 2/2)");
    service::set_running(TEST_SERVICE, false).expect("stop");
    assert!(!service::is_active(TEST_SERVICE));
    if was_active {
        let _ = service::set_running(TEST_SERVICE, true);
    }
}

#[test]
#[ignore]
fn root_service_disable_then_enable() {
    use crate::core::service;
    assert!(service::is_enabled(TEST_SERVICE), "test expects {TEST_SERVICE} to be enabled");
    println!("services: disabling {TEST_SERVICE} at boot (password prompt 1/2)");
    service::set_enabled(TEST_SERVICE, false).expect("disable");
    assert!(!service::is_enabled(TEST_SERVICE));
    println!("services: enabling it again (password prompt 2/2)");
    service::set_enabled(TEST_SERVICE, true).expect("enable");
    assert!(service::is_enabled(TEST_SERVICE));
    // the list shown in the UI agrees
    assert!(service::list().iter().any(|s| s.name == TEST_SERVICE && s.enabled));
}

#[test]
#[ignore]
fn root_search_as_root() {
    use crate::core::search::{self, Query};
    let q = Query { directory: "/root".into(), file_type: "d".into(), ..Default::default() };
    let (as_user, _) = search::run(&q).unwrap();
    println!("search: listing directories in /root as root (password prompt 1/1)");
    let (as_root, found) = search::run(&Query { as_root: true, ..q }).expect("find as root");
    println!("search: {as_user} as user, {as_root} as root");
    assert!(as_root >= as_user);
    assert!(found.iter().all(|f| f.path.starts_with("/root")));
    // metadata must come from the root search, not from a later stat as the user
    assert!(found.iter().all(|f| !f.user.is_empty() && f.modified > 0), "missing owner/time in root results");
    println!("search: owners {:?}", found.iter().map(|f| f.user.as_str()).collect::<std::collections::BTreeSet<_>>());
}

/// Removes a small leaf package and reinstalls it from a .deb.
/// KAPTAN_TEST_PKG (default "ncdu") and KAPTAN_TEST_DEB (the package's .deb, downloaded if unset).
#[test]
#[ignore]
fn root_package_remove_then_reinstall() {
    use crate::core::{command, package};
    let pkg = std::env::var("KAPTAN_TEST_PKG").unwrap_or_else(|_| "ncdu".into());
    let status = |p: &str| command::exec("dpkg-query", &["-W", "-f=${Status} ${Version}", p]).unwrap_or_default();
    let before = status(&pkg);
    assert!(before.starts_with("install ok installed"), "{pkg} must be installed: {before}");
    let manual_before = command::exec("apt-mark", &["showmanual", &pkg]).unwrap_or_default() == pkg;

    let deb = std::env::var("KAPTAN_TEST_DEB").unwrap_or_else(|_| {
        let dir = std::env::temp_dir().join("kaptan-test-deb");
        std::fs::create_dir_all(&dir).unwrap();
        std::process::Command::new("apt-get").args(["download", &pkg]).current_dir(&dir).status().unwrap();
        crate::core::fsutil::list_dir(&dir, true).into_iter().find(|p| p.extension().is_some_and(|e| e == "deb")).expect(".deb").display().to_string()
    });

    // the dry run must not touch anything else
    assert_eq!(package::simulate_removal(&[pkg.clone()]).unwrap(), vec![pkg.clone()]);

    println!("packages: removing {pkg} (password prompt 1/2)");
    package::uninstall(&[pkg.clone()]).expect("uninstall");
    assert!(!status(&pkg).starts_with("install ok installed"), "{pkg} still installed");
    assert!(!package::packages().contains(&pkg), "uninstaller list still shows {pkg}");

    println!("packages: reinstalling {pkg} from {deb} (password prompt 2/2)");
    command::sudo_exec("apt-get", &["install", "-y", &deb], None).expect("reinstall");
    assert_eq!(status(&pkg), before, "{pkg} not restored to the same version");
    let manual_after = command::exec("apt-mark", &["showmanual", &pkg]).unwrap_or_default() == pkg;
    assert_eq!(manual_after, manual_before, "manual/auto mark changed");
}
