use super::ConnectionPolicy;
use crate::util::try_run_command;
use crate::NumngError;
use std::path::PathBuf;
use std::process::Command;

const HEX_CHARS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
];

pub fn get_package_fs_basepath(
    source_uri: &String,
    git_ref: &String,
    base_dir: &PathBuf,
    connection_policy: ConnectionPolicy,
) -> Result<PathBuf, NumngError> {
    log::debug!("get_git_fs_basepath: {}", source_uri);

    // let base_path: PathBuf = base_dir
    //     .join("store/git")
    //     .join(crate::util::filesystem_safe(
    //         source_uri.split_once("://").unwrap().1.chars(),
    //     ));
    let base_path: PathBuf = base_dir.join("store/git").join(
        source_uri
            .split_once("://")
            .unwrap()
            .1
            .split("/")
            .into_iter()
            .map(|i| crate::util::filesystem_safe(i.chars()))
            .filter(|i| i.chars().into_iter().all(|i| i == '.')) // remove "", ".", and ".." to prevent overwriting something else
            .collect::<Vec<String>>()
            .join("/"),
    );

    let ref_path: PathBuf = base_path.join(crate::util::filesystem_safe(git_ref.chars()));

    if connection_policy == ConnectionPolicy::Offline {
        return Ok(ref_path);
    }

    if !base_path.exists() {
        std::fs::create_dir_all(&base_path).map_err(|e| NumngError::IoError(e))?;
    }
    let bare_path: PathBuf = base_path.join("__bare__");
    if !bare_path.exists() {
        git_clone(source_uri.as_str(), &base_path)?;
    }
    if ref_path.exists() {
        if connection_policy == ConnectionPolicy::Update {
            git_update_ref_dir(&ref_path, &git_ref)?;
        }
    } else {
        init_git_worktree(&ref_path, &git_ref, &bare_path)?;
    }

    Ok(ref_path)
}

fn git_update_ref_dir(ref_path: &PathBuf, git_ref: &String) -> Result<(), NumngError> {
    log::info!(
        "updating git worktree at {}",
        ref_path.to_str().expect("$HOME is not UTF-8?")
    );

    try_run_command(
        &mut Command::new("git")
            .arg("clean")
            .arg("--force")
            .arg("-d") // recurse into untracked directories  (has no long form -> description here)
            .arg("-x") // don’t use the standard ignore rules
            .arg("-e") // exclude <dir>
            .arg("/target")
            .current_dir(&ref_path),
    )?;
    try_run_command(
        &mut Command::new("git")
            .arg("fetch")
            .arg("origin")
            .arg(&git_ref)
            .current_dir(&ref_path),
    )?;
    try_run_command(
        &mut Command::new("git")
            .arg("reset")
            .arg("--hard")
            .arg("FETCH_HEAD")
            .current_dir(&ref_path),
    )?;
    Ok(())
}

fn git_clone(source_uri: &str, base_path: &PathBuf) -> Result<(), NumngError> {
    log::info!("git cloning {}", &source_uri);
    try_run_command(
        &mut Command::new("git")
            .arg("clone")
            .arg("--bare")
            .arg("--depth=1")
            .arg(source_uri)
            .arg("__bare__")
            .current_dir(base_path),
    )
}

fn init_git_worktree(
    ref_path: &PathBuf,
    git_ref: &String,
    bare_path: &PathBuf,
) -> Result<(), NumngError> {
    log::info!(
        "creating new git worktree at {}",
        ref_path.to_str().expect("$HOME is not UTF-8?")
    );
    match try_run_command(
        &mut Command::new("git")
            .arg("fetch")
            .arg("--depth=1")
            .arg("--tags")
            .arg("origin")
            .arg(&git_ref) // TODO: escape it? (what if it starts with "--")
            .current_dir(&bare_path),
    ) {
        Ok(()) => (),
        Err(NumngError::ExternalCommandExitcode { .. })
            if git_ref.chars().all(|c| HEX_CHARS.contains(&c)) =>
        {
            log::debug!(
                "failed git fetch, attempting unshallow since the git_ref looks like a short-hash"
            );
            try_run_command(
                &mut Command::new("git")
                    .arg("fetch")
                    .arg("--unshallow")
                    .current_dir(&bare_path),
            )?;
        }
        Err(e) => return Err(e),
    };

    try_run_command(
        &mut Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg(&ref_path)
            .arg(&git_ref)
            .current_dir(&bare_path),
    )
}
