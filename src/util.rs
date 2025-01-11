use std::{path::PathBuf, process::Command, str::Chars};

use crate::NumngError;

const FILESYSTEM_SAFE_CHARACTERS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '_', '-', '.', ' ',
];
pub fn filesystem_safe(text: Chars<'_>) -> String {
    text.into_iter()
        .map(|c| {
            if FILESYSTEM_SAFE_CHARACTERS.contains(&c) {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
}

pub fn try_run_command(command: &mut Command) -> Result<(), NumngError> {
    let output = match command.output() {
        Ok(o) => o,
        Err(e) => return Err(NumngError::ExternalCommandIO(e)),
    };
    if output.status.success() {
        Ok(())
    } else {
        Err(NumngError::ExternalCommandExitcode {
            command: format!("{:?}", command),
            stdout: String::from_utf8(output.stdout.clone())
                .unwrap_or_else(|_| format!("0x{:x?}", output.stdout)),
            stderr: String::from_utf8(output.stderr.clone())
                .unwrap_or_else(|_| format!("0x{:x?}", output.stderr)),
            exitcode: output.status.code().unwrap_or(-1),
        })
    }
}

pub fn symlink(from_path: &PathBuf, to_path: &PathBuf) -> Result<(), NumngError> {
    log::trace!(
        "symlink: {} -> {}",
        from_path.as_os_str().to_str().unwrap(),
        to_path.as_os_str().to_str().unwrap()
    );

    // completely untested since i have no windows
    #[cfg(target_os = "windows")]
    if from_path.is_file() {
        std::os::windows::fs::symlink_file(from_path, to_path)
    } else {
        std::os::windows::fs::symlink_dir(from_path, to_path)
    }
    .map_err(|err| NumngError::IoError(err))?;

    // not sure if this works on apple, but it should be "unix" ?
    #[cfg(not(target_os = "windows"))]
    std::os::unix::fs::symlink(from_path, to_path).map_err(|err| NumngError::IoError(err))?;

    Ok(())
}
