# bones-kernel-launch

Start a program in a folder, detached from the application, or open a terminal there. An "Open in VS Code / Terminal / Commits" button and an external diff tool are both this.

A kernel library with no bus, reached through `bones-kernel`'s `launch` feature as `bones_kernel::launch`. Both of its callers are native code, and a guest-facing "start any program" action would reach past every boundary the sandbox draws ([ADR-034](../../../docs/adr/ADR-034-os-capabilities-are-an-optional-module-splittable-per-capability.md)). Tool configuration, argument templates such as `{path}`, and settings stay in the application, which passes a finished `Program` here.

## API

| Item | Does |
| --- | --- |
| `resolve(program)` | Finds a program as a shell would, without starting it: `~` expanded, an explicit path taken as given, a bare name searched on `PATH`, and on Windows every `PATHEXT` extension tried, so `code` finds `code.cmd`. Use it to tell whether a tool is installed |
| `launch(&Program)` | Resolves and starts the program detached, in `Program::cwd`, with `Program::env` added to the inherited environment. `Launched::NotFound { searched }` when nothing resolves; `Err` when it was found and could not start |
| `open_terminal(dir, run)` | Opens a terminal in `dir`. With `run`, the terminal runs that command and stays open afterwards |

A started program gets no standard streams, its own process group on Unix, and a background thread that reaps it, so closing the application never closes what it started and a finished child never lingers.

## Platforms

| Platform | `.cmd` / `.bat` | Terminal |
| --- | --- | --- |
| Windows | Run through `cmd /C`, the only way Windows runs them, with the interpreter's console window hidden (`CREATE_NO_WINDOW`) | Windows Terminal (`wt -d <dir>`) when installed, otherwise `cmd` through `start` |
| Linux | — | `$TERMINAL`, then what `x-terminal-emulator` resolves to, then the first installed of: cosmic-term, gnome-terminal, ptyxis, konsole, kitty, alacritty, wezterm, foot, xfce4-terminal, xterm. With none installed, the error lists them all |
| macOS | — | Terminal.app, through `open -a Terminal`, or AppleScript when running a command |

The console window applies to the shim's interpreter only. A program resolved to an `.exe` starts as itself: a GUI program opens no console, and a console program gets the console Windows gives it.

## Environment

Children inherit the process environment unchanged, plus `Program::env`. The engine selects its X11 display backends without writing the environment, so a child sees the user's own `GDK_BACKEND` and `SDL_VIDEO_DRIVER` with nothing to restore. An application that still overrides its own variables must pass the user's values in `env` itself.

## Verification

The `.cmd` path is covered by a test that starts a shim through `cmd /C` and reads back a file it wrote with a variable from `env`. That the shim's console stays hidden is the `CREATE_NO_WINDOW` flag on the interpreter; no automated test can observe a window that never appears.
