//! Launch the host's preferred file manager without inheriting AppImage libraries.
use std::{
    env,
    ffi::{OsStr, OsString},
    io,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn host_paths(value: &OsStr, app_dir: &Path) -> io::Result<Option<OsString>> {
    let paths: Vec<_> = env::split_paths(value)
        .filter(|path| !path.starts_with(app_dir))
        .collect();
    if paths.is_empty() {
        Ok(None)
    } else {
        env::join_paths(paths)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }
}

fn host_command(program: &str) -> io::Result<Command> {
    let mut command = Command::new(program);
    // Only modify the child's environment, never the launcher's global environment.
    if let Some(app_dir) = env::var_os("APPDIR").filter(|v| !v.is_empty()) {
        let app_dir = Path::new(&app_dir);
        if !app_dir.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "APPDIR must be absolute",
            ));
        }
        for key in [
            "PATH",
            "LD_LIBRARY_PATH",
            "XDG_DATA_DIRS",
            "QT_PLUGIN_PATH",
            "QT_QPA_PLATFORM_PLUGIN_PATH",
            "GTK_PATH",
            "GI_TYPELIB_PATH",
            "GIO_EXTRA_MODULES",
            "GTK_DATA_PREFIX",
            "GTK_EXE_PREFIX",
            "GSETTINGS_SCHEMA_DIR",
            "GTK_IM_MODULE_FILE",
            "GDK_PIXBUF_MODULE_FILE",
        ] {
            if let Some(value) = env::var_os(key) {
                match host_paths(&value, app_dir)? {
                    Some(value) => {
                        command.env(key, value);
                    }
                    None => {
                        command.env_remove(key);
                    }
                }
            }
        }
    }
    // A launcher wrapper may isolate its data/config directories. External desktop
    // applications must use the original session settings (not the launcher's).
    // Empty saved values mean the variable was unset; use the desktop default.
    for key in [
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_DIRS",
        "GTK_THEME",
        "GDK_BACKEND",
    ] {
        if let Some(value) = env::var_os(format!("NRC_HOST_{}", key)) {
            if value.is_empty() {
                command.env_remove(key);
            } else {
                command.env(key, value);
            }
        }
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    Ok(command)
}

fn launch(command: &mut Command) -> io::Result<()> {
    let mut child = command.spawn()?;
    // xdg-open may stay alive for as long as the file manager window does.
    // Catch immediate failures, but neither block forever nor kill an open window.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("exited with {}", status),
                ))
            };
        }
        if Instant::now() >= deadline {
            thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
}

pub fn open_directory(directory: &Path) -> io::Result<()> {
    // Absolute paths also prevent leading '-' in names being parsed as options.
    let directory = directory.canonicalize()?;
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Path is not a directory",
        ));
    }
    let mut failures = Vec::new();
    // Respect the desktop's default handler; do not hard-code Dolphin/Nautilus/etc.
    for (program, args) in [("xdg-open", &[][..]), ("gio", &["open"][..])] {
        let mut command = host_command(program)?;
        command.args(args).arg(&directory);
        match launch(&mut command) {
            Ok(()) => return Ok(()),
            Err(error) => failures.push(format!("{}: {}", program, error)),
        }
    }
    Err(io::Error::new(io::ErrorKind::Other, format!(
        "Could not open directory {} ({}). Check the default handler for inode/directory and that xdg-utils or gio is installed.",
        directory.display(), failures.join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_bundle_paths_but_preserves_host_and_similar_names() {
        assert_eq!(
            host_paths(
                OsStr::new("/tmp/app/usr/lib:/usr/lib:/tmp/app-other/lib:/home/user/lib"),
                Path::new("/tmp/app")
            )
            .unwrap(),
            Some(OsString::from("/usr/lib:/tmp/app-other/lib:/home/user/lib"))
        );
    }

    #[test]
    fn removes_bundle_only_variable() {
        assert_eq!(
            host_paths(
                OsStr::new("/tmp/app//usr/share/schemas"),
                Path::new("/tmp/app")
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn reports_failed_process_instead_of_successful_spawn() {
        let error = launch(Command::new("/bin/sh").args(["-c", "exit 7"])).unwrap_err();
        assert!(error.to_string().contains('7'));
    }

    #[test]
    fn accepts_successful_process() {
        launch(Command::new("/bin/sh").args(["-c", "exit 0"])).unwrap();
    }

    #[test]
    fn rejects_missing_directory_before_launching() {
        assert!(open_directory(Path::new("/nonexistent/norisk-test-directory")).is_err());
    }
}
