use anyhow::{Result, bail};

const APP: &str = "GoXLR App.exe";
const BETA: &str = "GoXLR Beta App.exe";
const UTIL: &str = "goxlr-daemon.exe";
const UTIL_LINUX: &str = "goxlr-daemon";

pub fn status_check() -> Result<()> {
    let mut system = None;

    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};
    let refresh_kind = RefreshKind::new().with_processes(ProcessRefreshKind::new().with_user(UpdateKind::Always));
    system.replace(System::new_with_specifics(refresh_kind));

    if let Some(system) = &mut system {
        system.refresh_processes();
        let count = system.processes_by_exact_name(UTIL_LINUX).count();
        if count > 0 {
            bail!("Stop the Utility First!");
        }

        let count = system.processes_by_exact_name(UTIL).count();
        if count > 0 {
            bail!("Stop the Utility First!");
        }

        if system.processes_by_exact_name(APP).count() > 0 {
            bail!("Stop the Official App First!")
        }

        if system.processes_by_exact_name(BETA).count() > 0 {
            bail!("Stop the Official Beta App First!")
        }
    } else {
        bail!("Unable to Read System Processes, failing Pre-Flight");
    }

    Ok(())
}
