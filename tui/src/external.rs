use std::{
    ffi::OsStr,
    io,
    path::Path,
    process::{Command, Stdio},
};

/// Open a configuration file in the user's configured editor. Terminal
/// editors are launched in a new terminal so they do not take over the TUI.
pub fn open_config(path: &Path) -> io::Result<()> {
    let Some(mut command) = editor_command_from_environment(path)? else {
        return open::that_detached(path);
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

pub fn open_detached(target: impl AsRef<OsStr>) -> io::Result<()> {
    open::that_detached(target)
}

fn editor_command_from_environment(path: &Path) -> io::Result<Option<Command>> {
    let editor = std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });
    let Some(editor) = editor else {
        return Ok(None);
    };
    let terminal = std::env::var("TERMINAL")
        .ok()
        .filter(|value| !value.trim().is_empty());
    build_editor_command(path, &editor, terminal.as_deref()).map(Some)
}

fn build_editor_command(path: &Path, editor: &str, terminal: Option<&str>) -> io::Result<Command> {
    let mut editor_parts = split_command(editor, "EDITOR/VISUAL")?;
    let editor_program = editor_parts.remove(0);

    if is_terminal_editor(&editor_program) {
        let terminal = terminal.unwrap_or("xdg-terminal-exec");
        let mut terminal_parts = split_command(terminal, "TERMINAL")?;
        let terminal_program = terminal_parts.remove(0);
        let uses_xdg_terminal = executable_name(&terminal_program) == "xdg-terminal-exec";
        let mut command = Command::new(terminal_program);
        command.args(terminal_parts);
        if uses_xdg_terminal {
            command.arg("--");
        } else {
            command.arg("-e");
        }
        command.arg(editor_program).args(editor_parts).arg(path);
        Ok(command)
    } else {
        let mut command = Command::new(editor_program);
        command.args(editor_parts).arg(path);
        Ok(command)
    }
}

fn split_command(command: &str, variable: &str) -> io::Result<Vec<String>> {
    let parts = shlex::split(command).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{variable} contains invalid quoting"),
        )
    })?;
    if parts.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{variable} is empty"),
        ));
    }
    Ok(parts)
}

fn executable_name(program: &str) -> &str {
    Path::new(program)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(program)
}

fn is_terminal_editor(program: &str) -> bool {
    matches!(
        executable_name(program),
        "vi" | "vim" | "nvim" | "nano" | "micro" | "hx" | "helix" | "kak" | "kakoune"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_editor_uses_a_new_terminal() {
        let command = build_editor_command(
            Path::new("/tmp/clouds config.toml"),
            "nvim -f",
            Some("xdg-terminal-exec"),
        )
        .expect("command should be valid");

        assert_eq!(command.get_program(), "xdg-terminal-exec");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["--", "nvim", "-f", "/tmp/clouds config.toml"]
        );
    }

    #[test]
    fn graphical_editor_is_launched_directly() {
        let command = build_editor_command(
            Path::new("/tmp/tui-config.toml"),
            "code --new-window",
            Some("ghostty"),
        )
        .expect("command should be valid");

        assert_eq!(command.get_program(), "code");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["--new-window", "/tmp/tui-config.toml"]
        );
    }
}
