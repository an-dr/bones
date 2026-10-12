/// How one Linux terminal takes a working directory and a command.
pub(super) struct KnownTerminal {
    pub(super) program: &'static str,
    pub(super) dir_flag: &'static [&'static str],
    pub(super) exec_flag: &'static [&'static str],
}

/// Order is preference when several are installed and neither `$TERMINAL`
/// nor `x-terminal-emulator` names one.
pub(super) const KNOWN: &[KnownTerminal] = &[
    KnownTerminal {
        program: "cosmic-term",
        dir_flag: &["-w"],
        exec_flag: &["-e"],
    },
    KnownTerminal {
        program: "gnome-terminal",
        dir_flag: &["--working-directory"],
        exec_flag: &["--"],
    },
    KnownTerminal {
        program: "ptyxis",
        dir_flag: &["--new-window", "--working-directory"],
        exec_flag: &["--"],
    },
    KnownTerminal {
        program: "konsole",
        dir_flag: &["--workdir"],
        exec_flag: &["-e"],
    },
    KnownTerminal {
        program: "kitty",
        dir_flag: &["--directory"],
        exec_flag: &[],
    },
    KnownTerminal {
        program: "alacritty",
        dir_flag: &["--working-directory"],
        exec_flag: &["-e"],
    },
    KnownTerminal {
        program: "wezterm",
        dir_flag: &["start", "--cwd"],
        exec_flag: &["--"],
    },
    KnownTerminal {
        program: "foot",
        dir_flag: &["-D"],
        exec_flag: &[],
    },
    KnownTerminal {
        program: "xfce4-terminal",
        dir_flag: &["--working-directory"],
        exec_flag: &["-x"],
    },
    // No directory flag: it inherits the working directory set on spawn.
    KnownTerminal {
        program: "xterm",
        dir_flag: &[],
        exec_flag: &["-e"],
    },
];
