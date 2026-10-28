use std::path::{Path, PathBuf};

/// Finds `program` the way a shell would, without starting it.
///
/// - `~/` or `~\` at the start expands to the home folder.
/// - A path with more than one component, or an absolute one, is taken as
///   given.
/// - A bare name is searched on `PATH`.
/// - On Windows a name without an extension is tried with every `PATHEXT`
///   extension before it is tried as given, the way `cmd` does, so `code`
///   finds `code.cmd` rather than the extensionless shell script VS Code
///   ships next to it, which Windows cannot start.
///
/// Public so an application can tell whether a tool is installed before
/// offering it.
pub fn resolve(program: &str) -> Option<PathBuf> {
    let program = program.trim();
    if program.is_empty() {
        return None;
    }
    let expanded = match program
        .strip_prefix("~/")
        .or_else(|| program.strip_prefix("~\\"))
    {
        Some(rest) => dirs::home_dir()?.join(rest),
        None => PathBuf::from(program),
    };
    let extensions = executable_extensions(&expanded);
    let with_extension = |base: &Path| -> Option<PathBuf> {
        extensions
            .iter()
            .map(|ext| append_extension(base, ext))
            .find(|candidate| candidate.is_file())
    };
    if expanded.components().count() > 1 || expanded.is_absolute() {
        return with_extension(&expanded);
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .find_map(|dir| with_extension(&dir.join(&expanded)))
}

/// The suffixes to try on `program`: only the name as given outside Windows
/// or when it already has an extension; otherwise each `PATHEXT` extension
/// first and the bare name last.
pub(crate) fn executable_extensions(program: &Path) -> Vec<String> {
    if !cfg!(windows) || program.extension().is_some() {
        return vec![String::new()];
    }
    let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    pathext
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(str::to_lowercase)
        .chain(std::iter::once(String::new()))
        .collect()
}

fn append_extension(base: &Path, ext: &str) -> PathBuf {
    let mut name = base.as_os_str().to_owned();
    name.push(ext);
    PathBuf::from(name)
}
