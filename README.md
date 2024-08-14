# NU(-shell) MaNaGer

**This project is in a experimental stage.**  
Expect: breaking changes, bad ui, random crashes, etc.


## "working" features

* Package management for both the shell and projects
* Package and Registry compatibility with other managers:
  * own numng format
  * (mostly?) [nupm][] compatible (nupm is not in a final state and not fully documented)
  * (partially) [packer.nu][] compatible (exact filepaths differ, post install is missing, etc)
* Nu-Plugin management (installation, updating, etc)

And all this with:

* [Nix-store](https://en.wikipedia.org/wiki/Nix_(package_manager)) inspired multi-versioning based on [git worktrees](https://git-scm.com/docs/git-worktree)
* Independence from nu-script's breaking changes resulting in compatibiliy with both LTS and edge distros
* OS independence (windows, mac, linux, bsd)


## Usage / Quickstart

### Installation

Install the dependencies (`nushell`, `python3`, and `git`)

```nu
http get --raw "https://raw.githubusercontent.com/Jan9103/numng/main/numng.py" | save -r numng.py
python3 numng.py --nu-config init  # generate a basic config
rm numng.py  # no longer needed since it now installs and updates itself
nu  # open a new nu session to reload the config
```

Note: All numng managed binaries (including itself) are only available within nushell since it uses its own PATH entry.

In case anything goes wrong:
* removing the `source` line from your nu config completly disables numng
* the `numng.py` can be found at `~/.local/share/nushell/numng/git/github/jan9103/numng/main/numng.py`
* complete removal: `rm -r ~/.local/share/nushell/numng ~/.config/nushell/numng` and remove the `source` line from your nu config


### As a nu SHELL package manager

Example configuration (`~/.config/nushell/numng/numng.json`):

```json
{
  "name": "nu-config",
  "allow_build_commands": true,
  "depends": [
    {"name": "nu-themes"},
    {"name": "nu_plugin_file", "nu_plugins": ["target/release/nu_plugin_file"], "build_command": "cargo build --release",
      "source_uri": "https://github.com/fdncred/nu_plugin_file", "package_format": "numng"}
  ],
  "registry": [
    {"source_uri": "https://github.com/nushell/nupm", "package_format": "nupm"}
  ]
}
```

Applying the config: `numng --nu-config build` or `numng -n b`

Updating installed packages: `numng --nu-config build --pull-updates` or `numng -n b -u`

**Note:** For better [packer.nu][] compatability include the top-level-dependency `{"name": "packer.nu", "source_uri": "https://github.com/jan9103/packer.nu"}`


### As a project package manager

Create a `numng.json` in your project (or add `--package-file PATH` to all commands):  
(or generate it using `numng init`)

```json
{
  "name": "project-name",
  "depends": [
    {"name": "nu-scripts"}
  ],
  "linkin": {
    "lib/clippy": {"name": "nu-clippy"}
  },
  "registry": [
    {"source_uri": "https://github.com/nushell/nupm", "package_format": "nupm"}
  ]
}
```

**Dependencies:**  
If you want to keep your project nupm compatible use `depends` and `$env.NUPM_HOME` for dependencies.  
If thats not a concern `linkin` is more reliable (always the right version, easier to use, no need for overlays, etc).  
Both are explained below in `Nupm package format`

**Command:**  
`numng build` (short: `numng b`) is the base command.

If you want to update the packages add `--pull-updates` (short: `-u`) to the command

If you use `depends` for your dependencies or if one exports/.. CLI commands you have to use one of two options:

* add `--script-file script.nu` (short: `-s script.nu`) and activae it using `source script.nu`.
* add `--overlay-file overlay.nu` (short: `-o overlay.nu`) and activate it using `overlay use overlay.nu`.


## Numng package format

The package should contain a `numng.json`. Example:

```json
{
  "name": "Example Package",
  "linkin": {
    "libs/nutils": {"name": "jan9103/nutils", "git_ref": "v0.1"}
  }
}
```

The `base package` is the `numng.json` you call the command on (your shell config, project config, etc) in contrast to the downloaded ones.

key            | type                    | description
:------------- | :---------------------- | :----------
name           | `string`                | name of the package (REQUIRED in dependencies, linkins, etc)
linkin         | `record[string, package]` | symlink a package into this package (the key is `[PATH_IN_PACKAGE:]PATH_HERE` (similar to `docker -v`))
source_type    | `string`                | type of the source (only `git` is supported so far) (default: `git`)
source_uri     | `string`                | from where does the package come (example: `ssh://github.com/foo/bar`, `http://github.com/foo/bar`, `file:///home/user/my_package`)
git_ref        | `string`                | git reference (tag, commit, or branch) to use (default: `main`)
path_offset    | `string`                | path of the package within the source (example: `nu-scripts` within <https://github.com/amtoine/scripts>)
depends        | `list[package or string] or package or string` | packages this package depends on
package_format | `string`                | format of the package (`numng`, `nupm`, or `packer`) (default: auto detect)

numng package specific keys:

key            | type                       | description
:------------- | :------------------------- | :----------
nu_plugins     | `list[path]`               | nu plugin files, which should get registered via `plugin add`
registry       | `list[package] or package` | (only in base package) packages containing a registry (registries require `package_format`)
nu_libs        | `record[string, path]`     | directories and files, which should get linked into a `$env.NU_LIB_DIRS` (string is the target name)
shell_config   | `record[str, list[path] or path]` | things to load into the shell config. available keys: `source`, `source_env`, `use`, and `use_all` (`use path *`)
bin            | `dict[str, path]`          | put a file into the path and make it executable (key is the name)
build_command  | `string`                   | build commands for the project (run as `nu -c $build_command` in the package directory) (examples: `cargo build --release`, `make`, `nu build_script.nu`)
allow_build_commands | `boolean`            | (only in base package) execute `build_command`s from other packages (default: `false`)

nupm package specific keys:

key     | type     | description
:------ | :------- | :----------
version | `semver` | version of the package (requires `"package_format": "nupm"`) (example: `^1.2.1`)


## FAQ

### Why Python?

I initially started it in rust, but im not confident in my rust skills
and ended up abandoning it for years at this point.  
A non-rust compiled language wouldn't be a good fit for the comunity in my opinion.  
Nu is a great language, but breaking changes, etc created a lot of issues for the
[predecessor][packer.nu] of this.  
Python is already installed on most devices and can be read by almost every programer.


### Why a single file?

I don't want to deal with packaging python since its a annoying mess.  
Also easier install, etc.


## Todo / Ideas / Plans

* lots and lots of testing
* complete numng package format
  * load modules (completions, etc) into nu-shell on shell-open
  * (IDEA) project status (alpha, improving, maintained, unmaintained/archived)
  * and more
* full nupm compatability
* good error messages and logs (instead of just crashing python via `assert`)
* install script for easy setup
* non git package sources (github releases, http get, etc)
* numng repositories
* cli package-mangement commands (`numng add/remove/search/..`)
* (IDEA) (external?) project sandboxed (podman/docker/..) test-framework
* (IDEA) (external?) compile packages/apps into a single file
* collecting "garbage"


[nupm]: https://github.com/nushell/nupm
[packer.nu]: https://github.com/jan9103/packer.nu
