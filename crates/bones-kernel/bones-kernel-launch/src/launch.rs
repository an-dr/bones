use std::path::Path;
use std::process::{Command, Stdio};

use crate::{resolve, Launched, Program};

/// Starts `program` detached: closing this application never closes what it
/// started.
///
/// - The program is resolved first ([`resolve`]); a name that resolves to
///   nothing is [`Launched::NotFound`], not an error.
/// - On Windows a `.cmd` or `.bat` — VS Code's `code.cmd` is the usual one —
///   runs through `cmd /C`, the only way Windows runs it, with the
///   interpreter's console window hidden.
/// - Standard streams go nowhere, and on Unix the child gets its own process
///   group, so a signal aimed at this application stays with it.
///
/// `Err` means the program was found and could not be started.
pub fn launch(program: &Program) -> Result<Launched, String> {
    let Some(resolved) = resolve(&program.program) else {
        return Ok(Launched::NotFound {
            searched: program.program.clone(),
        });
    };
    let mut command = if is_batch_file(&resolved) {
        let mut command = Command::new("cmd");
        command.arg("/C").arg(&resolved);
        hide_console(&mut command);
        command
    } else {
        Command::new(&resolved)
    };
    command
        .args(&program.args)
        .current_dir(&program.cwd)
        .envs(program.env.iter().map(|(name, value)| (name, value)));
    spawn_detached(command)
        .map(|()| Launched::Started)
        .map_err(|error| format!("could not start {}: {error}", resolved.display()))
}

/// Spawns `command` with no standard streams, in its own process group on
/// Unix, and reaps it in the background so a finished child never lingers.
pub(crate) fn spawn_detached(mut command: Command) -> std::io::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn is_batch_file(path: &Path) -> bool {
    cfg!(windows)
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
}

fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}
