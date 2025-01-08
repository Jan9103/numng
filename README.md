# NU(-shell) MaNaGer

A declarative package manager for nushell and nu-scripts.

**This project is in a experimental stage.**  
Expect: breaking changes, bad ui, random crashes, etc.

**TOC:**

* [Alternatives](#alternatives)
* [Usage](#usage)
  * [Installation](#installation)
  * [Usage as a shell package manager](#shell_pm)
  * [Usage as a project package manager / How to package your project](#proj_pm)
* [The package format](#numng_format)
  * [List of available repositories](#repos)
* [FAQ](#faq)

<a name="alternatives"></a>

## Comparison to alternatives

* Numng is declarative. Instead of running `install x; install y; uninstall z` you define the target state `x and y are installed` and numng gets you there.
* Numng supports multiple package formats:
  * own `numng` format
  * [nupm][] (most parts)
  * [packer.nu][] (most parts)
* Numng can have multiple versions of each package at the same time. This allows package A to use nutils v1.0 while package B uses nutils v2.3.
* Numng is written in python and therefore both OS and (mostly) nu-version independent. ([detailed explanation](#why_python))

### Alternatives:

**[nupm][]:**
* Created by the nushell team.
* Targets nushell nightly and might not work with latest.
* Also still in a "experimental stage".


<a name="usage"></a>

## Usage / Quickstart

<a name="installation"></a>

### Installation

Dependencies: `python3`, `nu`, and `git`.

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

<a name="shell_pm"></a>

### As a nu SHELL package manager

Example configuration (`~/.config/nushell/numng/numng.json`):

```json
{
  "name": "nu-config",
  "allow_build_commands": true,
  "depends": [
    {"name": "jan9103/numng"},
    {"name": "fdncred/nu_plugin_file"},
    {"name": "nushell/nu_scripts/theme/gruvbox-dark"},
    {"name": "jan9103/nu-snippets/integration/carapace"},
    {"name": "jan9103/nu-snippets/prompt/starship"}
  ],
  "registry": [
    {"source_uri": "https://github.com/Jan9103/numng_repo", "package_format": "numng", "path_offset": "repo"}
  ]
}
```

if something is not available in a registry you can define the package inline:

```json
{
  "depends": {
    {"name": "nu_plugin_file", "nu_plugins": ["target/release/nu_plugin_file"], "build_command": "cargo build --release",
      "source_uri": "https://github.com/fdncred/nu_plugin_file", "package_format": "numng"}
  }
}
```

Applying the config: `numng --nu-config build` or `numng -n b`

Updating installed packages: `numng --nu-config build --pull-updates` or `numng -n b -u`

**Note:** For better [packer.nu][] compatability include the top-level-dependency `{"name": "packer.nu", "source_uri": "https://github.com/jan9103/packer.nu"}`

**Note:** If you want to use `numng` installed binaries in other shells add `~/.local/share/nushell/numng/nu_config_nupm_home/bin` to their `PATH`.
With such a setup it is possible to install `nushell/nushell` using numng for automatic updates.


<a name="proj_pm"></a>

### As a project package manager / Packaging your project

Create a `numng.json` in your project (or add `--package-file PATH` to all commands):  
(or generate it using `numng init`)

```json
{
  "name": "project-name",
  "depends": [
    {"name": "1kinoti/stdx.nu"}
  ],
  "linkin": {
    "webserver:nulibs/webserver": {"name": "jan9103/webserver.nu"}
  },
  "registry": [
    {"source_uri": "https://github.com/Jan9103/numng_repo", "package_format": "numng", "path_offset": "repo"}
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


<a name="numng_format"></a>

## Numng package format

`base package` refers to the `numng.json` you call the command on (your shell config, project config, etc) in contrast to the downloaded ones.

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
ignore_registry| `boolean`               | Usually package definitions get auto-expanded using registries, which could end up messing something up. This disables it for this package (not recursive).
version        | `semver`                | Select a version (only applicable when using a registry) (example: `^1.2.1`) (explanation [below](#semver))

numng package specific keys:

key            | type                       | description
:------------- | :------------------------- | :----------
nu_plugins     | `list[path]`               | nu plugin files, which should get registered via `plugin add`
registry       | `list[package] or package` | (only in base package) packages containing a registry (registries require `package_format`)
nu_libs        | `record[string, path]`     | directories and files, which should get linked into a `$env.NU_LIB_DIRS` (string is the target name)
shell_config   | `record[str, list[path] or path]` | things to load into the shell config. available keys: `source`, `source_env`, `use`, and `use_all` (`use path *`)
bin            | `dict[str, path]`          | put a file into the path and make it executable (key is the name)
build_command  | `string`                   | build commands for the project (executed as `nu -c $build_command` in the package directory) (examples: `cargo build --release`, `make`, `nu build_script.nu`)
allow_build_commands | `boolean`            | (only in base package) execute `build_command`s from other packages (default: `false`)

<a name="semver"></a>

`semver` (not 100% [semver](https://semver.org/) compatible):

* Up to 3 numbers seperated by dots (`.`). Example: `1.0.0`, `1.2`, `3`.
* Missing parts mean `any`/`latest`. Example: `"depends": "mylib/1.2"` could use `mylib/1.2.3`
* Prefixes can be used to be more precise:
  * `^1.2.3` means `1.2.3` or newer, but older than `2.0.0`
  * `~1.2.3` means `1.2.3` or newer, but older than `1.3.0`
  * `<1.2.3` means anything older than `1.2.3`
  * `>1.2.3` means anything newer than `1.2.3`
* `latest` is a special version. Used in a repository its newer than anything else. Used in a depends its short for `>0`
* `[a-zA-Z]+` versions are possible and only used if specifically requested by the user. example usecase: `githead`, `experimental`


<a name="repos"></a>

### Available Repositories

(you can have multiple ones active as long as their naming schemes don't collide)

* [numng-official](https://github.com/Jan9103/numng_repo) ([overview][repo_overview])
  * **size:** over 700 packages (including over 450 themes).
  * **package-freshness:** all packages have a `git`-HEAD version available. The packages which do have versions get updated at least once per week.
  * **snippet for adding:** `{"source_uri": "https://github.com/Jan9103/numng_repo", "package_format": "numng", "path_offset": "repo"}`
  * **package names:**
    * `[author-name]/[repo-name]` if the repo contains only 1 package
    * `[author-name]/[repo-name]/[package-name]` if the repo contains multiple packages
  * **package format:** `numng` / mixed
* [nupm-official](https://github.com/nushell/nupm)
  * **size:** over 20 packages.
  * **package-freshness:** it only contains official package releases and is currently (2024-12-02) lagging multiple months behind resulting in every `nu_plugin` package beeing broken, etc.
  * **snippet for adding:** `{"source_uri": "https://github.com/nushell/nupm", "package_format": "nupm"}`
  * **package-names:** `[package-name]`
  * **package format:** `nupm` only


## Numng Package registry

Any numng package can be a registry. You just have to register it as such.  
The packages are defined by `[PACKAGE_NAME].json` files (with UNIX-style `/` directory seperation).  
These json files should contain a dictionary with a `semver` as key and a package definition
(same as in a `numng.json`) in its value.  
It is also possible to set fallback values for all versions by creating a version called `_`.  
A version-alias can be created by just putting the target version as string into the value of a version (example: `"latest": "0.8"`).


<a name="faq"></a>

## FAQ

<a name="why_python"></a>

### Why Python?

I initially started it in rust, but im not confident in my rust skills
and ended up abandoning it for years at this point.  
A non-rust compiled language wouldn't be a good fit for the comunity in my opinion.  
Nu is a great language, but breaking changes, etc created a lot of issues for the
[predecessor][packer.nu] of this (example: you had to update `packer` before doing a system
update in case `nu` updated and broke the old `packer` version).  
Python is already installed on most devices and can be read by almost every programer.

### Why JSON?

* Its available in pythons standard library, which makes installation a lot easier.
* `nuon` is pretty complex with its special datatypes and the last time i tried to parse it i gave up.
* I dislike like [parts](https://ruudvanasseldonk.com/2023/01/11/the-yaml-document-from-hell) of the yaml spec.
* `marshal`, `pickle`, `hmac`, and `ini` would pose issues outside of python.


### Why a single file?

I don't want to deal with packaging python since its a annoying mess.  
Also easier install, etc.


[nupm]: https://github.com/nushell/nupm
[packer.nu]: https://github.com/jan9103/packer.nu
[repo_overview]: https://jan9103.github.io/nushell_packages
