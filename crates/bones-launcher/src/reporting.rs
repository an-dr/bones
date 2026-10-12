/// Reports a launch failure to the invoking console or redirected stderr.
pub fn report_error(text: &str) {
    #[cfg(windows)]
    {
        const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
        const STD_ERROR_HANDLE: u32 = -12i32 as u32;
        extern "system" {
            fn AttachConsole(process_id: u32) -> i32;
            fn GetStdHandle(which: u32) -> isize;
        }
        let which = STD_ERROR_HANDLE;
        let redirected = {
            let handle = unsafe { GetStdHandle(which) };
            handle != 0 && handle != -1
        };
        if !redirected && unsafe { AttachConsole(ATTACH_PARENT_PROCESS) } != 0 {
            if let Ok(mut console) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                use std::io::Write;
                let _ = console.write_all(text.as_bytes());
                let _ = console.flush();
                return;
            }
        }
    }
    use std::io::Write;
    eprint!("{text}");
    let _ = std::io::stderr().flush();
}
