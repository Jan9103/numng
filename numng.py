#!/usr/bin/env python3
from copy import deepcopy
from dataclasses import dataclass
from os import path, makedirs, mkdir, symlink, listdir, stat as os_stat, chmod, unlink, environ
from queue import SimpleQueue
from shutil import rmtree
from sys import stdout, orig_argv
from typing import List, Dict, Optional, Any, Tuple, Iterable, Union
from tempfile import TemporaryDirectory
import json
import logging
import stat
import string
import subprocess


logger = logging.getLogger(__name__)
logger.setLevel(logging.DEBUG)
log_handler = logging.StreamHandler(stdout)
log_handler.setLevel(logging.INFO)
log_formatter = logging.Formatter('%(asctime)s::%(levelname)s: %(message)s')
log_handler.setFormatter(log_formatter)
logger.addHandler(log_handler)
del log_formatter


VALID_FILESYSTEM_CHARACTERS: str = "-_. %s%s" % (string.ascii_letters, string.digits)
BASEDIRECTORY: str = path.join(path.expanduser('~'), ".local", "share", "nushell", "numng")


class SemVer:
    def __init__(self, text: Optional[str]) -> None:
        text = text or ""
        c0: str = text[0] if len(text) != 0 else ""
        self.op: Optional[str] = c0 if c0 in "<>^~" else None
        numbers: List[int] = [
            int(a) for a in (
                "".join([i for i in section if i in string.digits])
                for section in text.split(".")
            ) if a != ""
        ]
        self.major = numbers[0] if len(numbers) != 0 else None
        self.minor = numbers[1] if len(numbers) > 1 else None
        self.patch = numbers[2] if len(numbers) > 2 else None
        if len(text) > 0 and all(i in string.ascii_letters for i in text):
            self.op = text

    def __eq__(self, other: Any) -> bool:
        if not isinstance(other, SemVer):
            return False
        if (self.op or '') not in "<>^~" or (other.op or '') not in "<>^~":
            return self.op == other.op
        if (
            self.major is None or other.major is None
            or ((self.op == ">" or other.op == "<") and self.major < other.major)
            or ((other.op == ">" or self.op == "<") and other.major < self.major)
        ):
            return True
        if self.major != other.major:
            return False
        if (
            self.minor is None or other.minor is None
            or ((self.op in (">", "^") or other.op == "<") and self.minor < other.minor)
            or ((other.op in (">", "^") or self.op == "<") and other.minor < self.minor)
        ):
            return True
        if self.minor != other.minor:
            return False
        return (
            self.patch is None or other.patch is None
            or self.patch == other.patch
            or ((self.op in (">", "^", "~") or other.op == "<") and self.patch < other.patch)
            or ((other.op in (">", "^", "~") or self.op == "<") and other.patch < self.patch)
        )

    def __gt__(self, other: Any) -> bool:
        if not isinstance(other, SemVer) or other.major is None:
            return True
        if self.op == "latest":
            return True
        if other.op == "latest":
            return False
        if self.major is None or self.major < other.major:
            return False
        if self.major > other.major or other.minor is None:
            return True
        if self.minor is None or self.minor < other.minor:
            return False
        if self.minor > other.minor or other.patch is None:
            return True
        return not (self.patch is None or self.patch < other.patch)

    def __str__(self) -> str:
        return f"SemVer({self.op or ''}{'.'.join([str(self.major or ''), str(self.minor or ''), str(self.patch or '')])})"

    def latest_matching_dict_entry(self, options: Dict[Union[str, "SemVer", None], Any]) -> Optional[Any]:
        biggest_available: Optional[Tuple[SemVer, Any]] = None
        for option in (((k if isinstance(k, SemVer) else SemVer(k)), v) for k, v in options.items() if k != "_"):
            if self.op == "latest" or self.__eq__(option[0]):
                if biggest_available is None or option[0].__gt__(biggest_available[0]):
                    biggest_available = option
        return None if biggest_available is None else biggest_available[1]


@dataclass(kw_only=True)
class Package:
    name: str
    depends: Optional[List["Package"]] = None
    source_type: Optional[str] = None
    source_uri: Optional[str] = None
    source_git_ref: Optional[str] = None
    source_path_offset: Optional[str] = None
    registries: Optional[List["Package"]] = None
    package_format: Optional[str] = None
    extra_data: Optional[Dict[str, Any]] = None

    def include_data(self, other: "Package") -> None:
        if self.depends is None:
            self.depends = other.depends
        if self.source_type is None:
            self.source_type = other.source_type
        if self.source_uri is None:
            self.source_uri = other.source_uri
        if self.source_git_ref is None:
            self.source_git_ref = other.source_git_ref
        if self.source_path_offset is None:
            self.source_path_offset = other.source_path_offset
        if self.package_format is None:
            self.package_format = other.package_format
        if other.extra_data:
            self.extra_data = {**other.extra_data, **(self.extra_data or {})}

class PackageRegistry:
    # why does pyright not have a option to disable unused variable? https://github.com/microsoft/pyright/blob/main/docs/configuration.md
    def get_by_name(self, name: str, version: Optional[str] = None, **_) -> Optional[Package]:
        raise NotImplementedError()


class NumngPackageRegistry(PackageRegistry):
    def __init__(self, registry_dir: str) -> None:
        self._registry_dir: str = registry_dir

    def get_by_name(self, name: str, version: Optional[str] = None, **_) -> Optional[Package]:
        filepath = path.join(self._registry_dir, *[filesystem_safe(i) for i in (name+".json").split("/") if i and i != ".."])
        if not path.isfile(path.join(filepath)):
            logger.debug(f"numng_registry: no package-name match found for {name}/{version}")
            return None
        with open(filepath, "r") as fp:
            try:
                version_dict = json.load(fp)
            except json.JSONDecodeError:
                assert False, f"The repository {self._registry_dir} contains a invalid json file at {filepath}"
        found_package = SemVer(version or "latest").latest_matching_dict_entry(version_dict)
        if found_package is None:
            logger.debug(f"numng_registry: no version match found for {name}/{version}")
            return None
        while isinstance(found_package, str):
            assert found_package in version_dict, f'Numng-Repo defined a invalid version alias from {name}/{version or "latest"} to {name}/{found_package}'
            found_package = version_dict[found_package]
        found_package["name"] = name
        result: Package = load_package_from_json(found_package, allow_no_name=False)
        if "_" in version_dict:
            result.include_data(load_package_from_json(version_dict["_"], allow_no_name=True))
        return result


class NupmPackageRegistry(PackageRegistry):
    def __init__(self, registry_dir: str) -> None:
        self._registry_dir: str = registry_dir
        with open(path.join(registry_dir, "registry.nuon"), "r") as fp:
            # git already checks hashes -> no need to use the hashes here
            self._packages: Dict[str, str] = {i["name"]: i["path"] for i in load_nuon(fp.read())}

    def get_by_name(self, name: str, version: Optional[str] = None, **_) -> Optional[Package]:
        if (package_details_path := self._packages.get(name)) is None:
            return None
        package_details_path = path.abspath(path.join(self._registry_dir, package_details_path))
        assert package_details_path.startswith(self._registry_dir), f"Package registry unsafe (attempted to access {package_details_path})"
        with open(package_details_path, "r") as fp:
            raw_file: str = fp.read()
        return load_nupm_package_from_registry_nuon(load_nuon(raw_file), name=name, version=version)


def _listify(i: Any) -> List[Any]:
    if i is None:
        return []
    return i if isinstance(i, list) else [i]


def load_nupm_package_from_registry_nuon(json_data: Any, name: Optional[str] = None, version: Optional[str] = None) -> Optional[Package]:
    assert isinstance(json_data, list), "Invalid package-file in nupm registry (not a list)"
    wanted_semver: SemVer = SemVer(version)
    biggest_available: Optional[Tuple[SemVer, Any]] = wanted_semver.latest_matching_dict_entry({i.get("version"): i for i in json_data if name in (None, i.get("name"))})
    if biggest_available is None:
        logger.debug(f"load_nupm_package_from_registry_nuon: no match found for {name}/{version}")
        return None
    package_nuon = biggest_available[1]
    return Package(
        name=package_nuon["name"],
        source_type=package_nuon.get("type"),
        source_uri=(package_nuon.get("info") or {}).get("url"),
        source_git_ref=(package_nuon.get("info") or {}).get("revision"),
        source_path_offset=package_nuon.get("path"),
        package_format="nupm",
        # version=package_nuon.get("version"),
    )


@dataclass(kw_only=True)
class LoaderScriptSnippet:
    name: str
    depends: List[str]
    snippet: str


def sort_loader_script_snippets(snippets: List[LoaderScriptSnippet]) -> List[str]:
    result: List[str] = []
    todo: List[LoaderScriptSnippet] = deepcopy(snippets)
    for snippet in todo:
        snippet.depends = [
            dep for dep in snippet.depends
            if any(True for i in todo if i.name == dep)
        ]
    last_len: int = len(todo)
    while len(todo) != 0:
        for snippet in todo:
            if len(snippet.depends) == 0:
                result.append(snippet.snippet)
                todo.remove(snippet)
                if any(True for i in todo if i.name == snippet.name):
                    continue
                for i in todo:
                    if snippet.name in i.depends:
                        i.depends.remove(snippet.name)
        assert last_len != len(todo), "Unable to sort load snippets (circular dependencies): " + " ".join(i.name for i in todo)
        last_len = len(todo)
    return result


class Loader:
    def __init__(
        self,
        numng_file_path: str,
        generate_script: Optional[str] = None,
        generate_overlay: Optional[str] = None,
        nupm_home: Optional[str] = None,
        delete_existing_nupm_home: bool = False,
        pull_updates: bool = False,
        handle_nu_plugins: bool = False,
        allow_build_commands: Optional[bool] = None,
    ) -> None:
        self._nupm_home: Optional[str] = nupm_home
        self._loader_script_snippets_env: List[LoaderScriptSnippet] = []
        self._loader_script_snippets_use: List[LoaderScriptSnippet] = []
        self._loader_script_snippets_script: List[LoaderScriptSnippet] = []
        self._registries: List[PackageRegistry] = []
        self._load_q: SimpleQueue[Tuple[Package, str]] = SimpleQueue()
        self._loaded: List[str] = []  # Basepath
        self._pull_updates: bool = pull_updates
        self._nu_plugin_paths: List[str] = []
        self._allow_build_commands: bool = allow_build_commands or False
        self._nupm_home_symlink_todo: List[Tuple[str, str]] = []

        if nupm_home is not None:
            assert len(path.normpath(nupm_home).strip(path.sep).split(path.sep)) > 2, f"Due to security reasons (danger of damaging /home/user or something) the NUPM_HOME cant be this close to the file-root ({nupm_home})"
            if path.exists(nupm_home):
                assert delete_existing_nupm_home, f"NUPM_HOME at {nupm_home} already exists and delete existing is off"

        logger.debug(f"loading initial base package from {numng_file_path}")
        with open(numng_file_path, "r") as fp:
            package: Package = load_package_from_json(json.load(fp), allow_no_name=True)
        for registry in (package.registries or []):
            self._load_registry(registry, self._download_package(registry))
        base_path: str = path.abspath(path.join(numng_file_path, path.pardir))
        self._load_q.put((package, base_path))

        self._allow_build_commands = ((package.extra_data.get("allow_build_commands") or False) if package.extra_data is not None else False) if allow_build_commands is None else allow_build_commands

        logger.debug("entering load_q loop")
        while not self._load_q.empty():
            package, base_path = self._load_q.get()
            if base_path in self._loaded:
                continue
            self._loaded.append(base_path)
            self._load_package(package, base_path)

        if generate_script is not None:
            logger.debug(f"generating script at {generate_script}")
            load_script: str = "\n".join([
                "export-env {",
                "$env.ENV_CONVERSIONS = ($env | get -i ENV_CONVERSIONS | default {} | upsert 'PATH' {|_| {'from_string': {|s| $s | split row (char esep)}, 'to_string': {|v| $v | str join (char esep)}}})",
                *([
                    f"$env.NUPM_HOME = {json.dumps(self._nupm_home)}",
                    "$env.NU_LIB_DIRS = ($env | get -i NU_LIB_DIRS | default []"
                    f" | append {json.dumps(path.join(self._nupm_home, 'modules'))}"
                    f" | append {json.dumps(path.join(self._nupm_home, 'overlays'))})",
                    f"$env.PATH = ($env.PATH | append {json.dumps(path.join(self._nupm_home, 'bin'))})",
                ] if self._nupm_home is not None else []),
                *sort_loader_script_snippets(self._loader_script_snippets_env),
                "}",
                *sort_loader_script_snippets([
                    *self._loader_script_snippets_use,
                    *self._loader_script_snippets_script,
                ]),
            ])
            with open(generate_script, "w") as fp:
                fp.write(load_script)
        if generate_overlay is not None:
            logger.debug(f"generating overlay at {generate_overlay}")
            overlay_script: str = "\n".join([
                "export-env {",
                *([f"$env.NUPM_HOME = {json.dumps(nupm_home)}"] if self._nupm_home is not None else []),
                *sort_loader_script_snippets(self._loader_script_snippets_env),
                "}",
                *sort_loader_script_snippets(self._loader_script_snippets_use),
            ])
            with open(generate_overlay, "w") as fp:
                fp.write(overlay_script)

        if nupm_home is not None:
            logger.debug(f"init nupm_home at {nupm_home}")
            if path.exists(nupm_home):
                rmtree(nupm_home)
            makedirs(path.join(nupm_home, "modules"))
            mkdir(path.join(nupm_home, "bin"))
            mkdir(path.join(nupm_home, "overlays"))

        for s1, s2 in self._nupm_home_symlink_todo:
            symlink(src=s1, dst=s2)

        if handle_nu_plugins:
            logger.debug(f"updating plugins")
            self._generate_nu_plugins()

    def _registry_get_by_name(self, name: str, **kwargs) -> Optional[Package]:
        for registry in self._registries:
            if (result := registry.get_by_name(name, **kwargs)):
                return result
        return None

    def _load_registry(self, package: Package, base_path: str) -> None:
        logger.debug(f"loading registry from {base_path}")
        if package.package_format == "nupm":
            assert path.exists(path.join(base_path, "registry", "registry.nuon")), "Failed to load nupm registry (registry/registry.nuon not found)"
            self._registries.append(NupmPackageRegistry(path.join(base_path, "registry")))
            return
        if package.package_format == "numng":
            self._registries.append(NumngPackageRegistry(base_path))
            return
        raise AssertionError("Failed to load registry (unknown or unsupported package_format)")

    def _register_nupm_module(self, module_name: str, module_source_path: str) -> None:
        if self._nupm_home is None:
            return
        dst: str = path.abspath(path.join(self._nupm_home, "modules", filesystem_safe(module_name)))
        assert dst.startswith(path.join(self._nupm_home, "modules"))
        self._nupm_home_symlink_todo.append((module_source_path, dst,))

    def _register_nupm_binary(self, binary_name: str, binary_source_path: str) -> None:
        if self._nupm_home is None:
            return
        dst: str = path.abspath(path.join(self._nupm_home, "bin", filesystem_safe(binary_name)))
        assert dst.startswith(path.join(self._nupm_home, "bin"))
        chmod(binary_source_path, os_stat(binary_source_path).st_mode | stat.S_IEXEC)
        self._nupm_home_symlink_todo.append((binary_source_path, dst,))

    def _register_nupm_overlay(self, overlay_name: str, overlay_source_path: str) -> None:
        if self._nupm_home is None:
            return
        dst: str = path.abspath(path.join(self._nupm_home, "overlays", filesystem_safe(overlay_name)))
        assert dst.startswith(path.join(self._nupm_home, "overlays"))
        self._nupm_home_symlink_todo.append((overlay_source_path, dst,))

    def _download_packages(self, packages: List[Package]) -> List[Tuple[Package, str]]:
        return [(package, self._download_package(package)) for package in packages]

    def _download_package(self, package: Package) -> str:
        if (
            self._registries
            and (not ((package.extra_data or {}).get("ignore_registry") == True))
            # and (package.source_type is None or package.source_uri is None)
            and (regpkg := self._registry_get_by_name(package.name, version=(package.extra_data or {}).get("version"))) is not None
        ):
            package.include_data(regpkg)
        assert package.source_uri is not None, f"Failed to download {package.name} (unknown source_uri)"
        base_path: Optional[str] = None
        if package.source_type in ("git", None):
            assert package.source_uri is not None, f"Failed to generate loader for {package.name} (missing uri)"
            base_path = get_git_ref_path(package.source_uri, package.source_git_ref, download=True, update=self._pull_updates)
            base_path = path.join(base_path, package.source_path_offset) if package.source_path_offset else base_path
        else:
            raise AssertionError(f"Failed to download {package.name} (unknown or unsupported source-type)")
        return base_path

    def _load_package(self, package: Package, base_path: str) -> None:
        for i in self._download_packages(package.depends or []):
            self._load_q.put(i)
        if package.package_format == "numng" or (package.package_format == None and path.isfile(path.join(base_path, "numng.json"))):
            fp: str = path.join(base_path, "numng.json")
            logger.info(f"Loading numng package {package.name}")
            self._load_numng(package, fp if path.isfile(fp) else None, base_path)
            return
        if package.package_format in ("nupm", None) and path.isfile(fp := path.join(base_path, "nupm.nuon")):
            logger.info(f"Loading nupm package {package.name}")
            self._load_nupm(package, fp, base_path)
            return
        if package.package_format in ("packer", "packer.nu") and path.isfile(fp := path.join(base_path, "meta.nuon")):
            logger.info(f"Loading packer.nu package {package.name}")
            self._load_packer_meta(package, fp, base_path)
            return
        logger.info(f"No specific load action for {package.name} ({package.package_format}) found.")

    def _load_packer_meta(self, package: Package, meta_nuon_path: str, base_path: str) -> None:
        with open(meta_nuon_path, "r") as fp:
            meta_nuon_str: str = fp.read()
        meta_nuon: Any = load_nuon(meta_nuon_str)
        del meta_nuon_str, meta_nuon_path
        assert isinstance(meta_nuon, dict), f"Invalid packer.nu meta.nuon in {package.name} (not a record)"
        for module in (meta_nuon.get("prefixed_modules") or []):
            pm_mod_path: str = path.join(base_path, *module.split("/"))
            assert pm_mod_path.startswith(base_path), f"Security error: {package.name}'s prefixed module paths invalid"
            self._loader_script_snippets_use.append(LoaderScriptSnippet(
                name=package.name,
                depends=[i.name for i in package.depends] if package.depends else [],
                snippet=f"export use {pm_mod_path}"
            ))
        for module in (meta_nuon.get("modules") or []):
            mod_path: str = path.join(base_path, *module.split("/"))
            assert mod_path.startswith(base_path), f"Security error: {package.name}'s unprefixed module paths invalid"
            self._loader_script_snippets_use.append(LoaderScriptSnippet(
                name=package.name,
                depends=[i.name for i in package.depends] if package.depends else [],
                snippet=f"export use {mod_path} *"
            ))
        if path.isfile(env_nu := path.join(base_path, "env.nu")):
            self._loader_script_snippets_env.append(LoaderScriptSnippet(
                name=package.name,
                depends=[i.name for i in package.depends] if package.depends else [],
                snippet=f"source-env {env_nu}",
            ))
        if path.isfile(init_nu := path.join(base_path, "init.nu")):
            self._loader_script_snippets_env.append(LoaderScriptSnippet(
                name=package.name,
                depends=[i.name for i in package.depends] if package.depends else [],
                snippet=f"use {init_nu} *",
            ))
        if path.isdir(lib_dir := path.join(base_path, "lib")):
            self._loader_script_snippets_env.append(LoaderScriptSnippet(
                name=package.name,
                depends=[i.name for i in package.depends] if package.depends else [],
                snippet=f"$env.NU_LIB_DIRS = ($env | get -i NU_LIB_DIRS | default [] | append {json.dumps(lib_dir)})",
            ))

    def _load_numng(self, package: Package, numng_json_path: Optional[str], base_path: str) -> None:
        if numng_json_path is not None:
            with open(numng_json_path, "r") as fp:
                numng_json: Dict[str, Any] = json.load(fp)
            assert isinstance(numng_json, dict), f"Invalid numng.json in {package.name} (not a dict)"
            for dependency in _listify(numng_json.get("depends")):
                dep_pkg: Optional[Package] = load_package_from_json(dependency)
                assert dep_pkg is not None, f"Package from numng.json in {package.name} not found ({dependency.get('name')})"
                self._load_q.put((dep_pkg, self._download_package(dep_pkg)))
        else:
            logger.debug("_load_numng: falling back to package.extra_data (numng_json_path is None)")
            numng_json = package.extra_data or {}
        if "build_command" in numng_json:
            assert self._allow_build_commands, f'package {package.name} contains a build_comamnd. to use this package you will have to allow those by adding `"allow_build_commands": true` to your config'
            logger.debug(f"Building {package.name}: {numng_json['build_command']}")
            build_proc = subprocess.run(["nu", "--no-config-file", "-c", numng_json['build_command']], cwd=base_path, stdout=subprocess.DEVNULL)
            assert build_proc.returncode == 0, f"build_command for {package.name} failed"
        if "linkin" in numng_json:
            assert isinstance(numng_json["linkin"], dict), f"Invalid numng.json in {package.name} (linkin not a dict)"
            for linkin_path, linkin_json in numng_json["linkin"].items():
                if ":" in linkin_path:
                    repo_path, linkin_path = linkin_path.split(":", 1)
                else:
                    repo_path = None
                assert (linkin_path := path.abspath(path.join(base_path, *(linkin_path.split("/"))))).startswith(base_path), f"Package tried to linkin outside of its own directory: {package.name} to {linkin_path}"
                linkin: Package = load_package_from_json(linkin_json)
                logger.debug(f"linkin: path={linkin_path} target={package.name} source={linkin.name}")
                linkin_base_path: str = self._download_package(linkin)
                if repo_path is not None:
                    assert (tmp := path.abspath(path.join(linkin_base_path, repo_path))).startswith(linkin_base_path), "Security issue: linkin package-rel-path is outside of package"
                    linkin_base_path = tmp
                if not path.exists(linkin_pardir := path.abspath(path.join(linkin_path, path.pardir))):
                    makedirs(linkin_pardir)
                if path.exists(linkin_path):
                    assert path.islink(linkin_path), f"Failed linkin at {linkin_path}: path exists and is not a symlink"
                    if path.realpath(linkin_path) == linkin_base_path:
                        continue
                    unlink(linkin_path)
                self._nupm_home_symlink_todo.append((linkin_base_path, linkin_path,))
        for plugin in _listify(numng_json.get("nu_plugins")):
            plugin_path: str = path.abspath(path.join(base_path, plugin))
            assert plugin_path.startswith(base_path), f"Security error: {package.name} tried to register a plugin outside of its directory"
            self._nu_plugin_paths.append(plugin_path)
        if "nu_libs" in numng_json:
            assert isinstance(numng_json["nu_libs"], dict), f"Invalid numng.json in {package.name} (nu_libs is not a dict)"
            for name, rel_path in numng_json["nu_libs"].items():
                abs_path: str = path.abspath(path.join(base_path, rel_path))
                assert abs_path.startswith(base_path), f"Security error: {package.name} tried to register a lib outside of its directory"
                logging.debug(f"Registered module {name} for {package.name}")
                self._register_nupm_module(module_name=name, module_source_path=abs_path)
        if (sc := numng_json.get("shell_config")) is not None:
            assert isinstance(numng_json["shell_config"], dict), f"Invalid numng.json in {package.name} (shell_config not a dict)"
            deps: List[str] = [i.name for i in package.depends or []]
            for src_file in _listify(sc.get("source")):
                logger.debug(f"source file found: {src_file}")
                assert (src_file := path.abspath(path.join(base_path, src_file))).startswith(base_path), f"Security error: {package.name} tried to register a shell_config outside of its directory"
                self._loader_script_snippets_script.append(LoaderScriptSnippet(name=package.name, depends=deps, snippet=f"source {json.dumps(src_file)}"))
            for use_file in _listify(sc.get("use")):
                logger.debug(f"use file found: {use_file}")
                assert (src_file := path.abspath(path.join(base_path, src_file))).startswith(base_path), f"Security error: {package.name} tried to register a shell_config outside of its directory"
                self._loader_script_snippets_use.append(LoaderScriptSnippet(name=package.name, depends=deps, snippet=f"export use {json.dumps(use_file)}"))
            for use_file in _listify(sc.get("use_all")):
                logger.debug(f"use_all file found: {use_file}")
                assert (use_file := path.abspath(path.join(base_path, use_file))).startswith(base_path), f"Security error: {package.name} tried to register a shell_config outside of its directory"
                self._loader_script_snippets_use.append(LoaderScriptSnippet(name=package.name, depends=deps, snippet=f"export use {json.dumps(use_file)} *"))
            for src_env_file in _listify(sc.get("source_env")):
                logger.debug(f"load_env file found: {src_env_file}")
                assert (src_env_file := path.abspath(path.join(base_path, src_env_file))).startswith(base_path), f"Security error: {package.name} tried to register a shell_config outside of its directory"
                self._loader_script_snippets_env.append(LoaderScriptSnippet(name=package.name, depends=deps, snippet=f"source-env {json.dumps(src_env_file)}"))
        if "bin" in numng_json:
            assert isinstance(numng_json["bin"], dict), f"Invalid numng.json in {package.name} (bin has to be a dict)"
            for name, rel_path in numng_json["bin"].items():
                bin_abs_path: str = path.abspath(path.join(base_path, *rel_path.split("/")))
                logger.debug(f"registering binary: {name} from {package.name}")
                assert bin_abs_path.startswith(base_path), f"Security error: {package.name} tried to register a binary outside of its path"
                self._register_nupm_binary(name, bin_abs_path)
        # TODO: modules, overlay, scripts, envs, config additions, etc

    def _load_nupm(self, package: Package, nupm_nuon_path: str, base_path: str) -> None:
        with open(nupm_nuon_path, "r") as fp:
            nupm_nuon_str: str = fp.read()
        nupm_nuon = load_nuon(nupm_nuon_str)
        del nupm_nuon_path, nupm_nuon_str
        assert isinstance(nupm_nuon, dict), f"invalid nupm.nuon in {package.name} (not a record)"
        assert "type" in nupm_nuon, f"invalid nupm.nuon in {package.name} (missing type)"
        if nupm_nuon["type"] == "module":
            assert "name" in nupm_nuon, f"invalid nupm.nuon in {package.name} (missing name)"
            assert path.exists(mod_dir_path := path.join(base_path, nupm_nuon["name"])), f"module-nupm-package {package.name} does not contain a module dir"
            self._register_nupm_module(nupm_nuon["name"], mod_dir_path)
        elif nupm_nuon["type"] == "script":
            if path.exists(script_path := path.join(base_path, script_name := f"{package.name}.nu")):
                self._register_nupm_binary(script_name, script_path)
        elif nupm_nuon["type"] == "custom":
            assert self._allow_build_commands is True, f"Cannot load nupm custom-type package {package.name} (allow_build_commands is false)"
            assert path.exists(build_script_path := path.join(base_path, "build.nu")), f"Invalid nupm custom-type package {package.name} (missing build.nu)"
            # im seriosly questioning my sanity here, but as far as i can see nupm runs the build script in a empty temporary directory and deletes the tmpdir
            # afterwards without using the tmpdir or giving the buildscript paths, etc
            with TemporaryDirectory() as tmpdir:
                logger.debug(f"Building {package.name} (nupm-custom)")
                build_proc = subprocess.run(["nu", "--no-config", build_script_path], cwd=tmpdir, stdout=subprocess.DEVNULL)
                assert build_proc.returncode == 0, f"nupm-custom build for {package.name} failed"
        else:
            raise AssertionError(f"Failed to load nupm-package {package.name} (unknown package type: {nupm_nuon['type']})")
        if "scripts" in nupm_nuon:
            assert isinstance(nupm_nuon["scripts"], list), f"Invalid nupm.nuon: scripts is supposed to be a list[str]. {package.name}"
            for script_subpath in nupm_nuon["scripts"]:
                abs_path: str = path.abspath(path.join(base_path, script_subpath))
                assert abs_path.startswith(base_path), f"Security issue: {package.name} tried to link {abs_path} as a script"
                self._register_nupm_binary(path.split(script_subpath)[1], abs_path)
        if "dependencies" in nupm_nuon:
            assert isinstance(nupm_nuon["dependencies"], list), f"Invalid nupm.nuon {package.name} (dependencies not a list)"
            nupm_deps: Optional[List[Tuple[str, str]]] = None
            if isinstance(nupm_nuon["dependencies"], list):
                nupm_deps = [
                    (tuple(dep.rsplit("/", 1)) if "/" in dep else (dep, None))  # type: ignore
                    for dep in nupm_nuon["dependencies"]
                ]
            elif isinstance(nupm_nuon["dependencies"], dict):
                nupm_deps = list(nupm_nuon["dependencies"].items())
            assert nupm_deps is not None, f"Nupm package {package.name}'s dependency list is neither a list nor a dict"
            for name, version in nupm_deps:
                if name == "nushell":  # some packages declare version compatability this way. its not a real package
                    continue
                dep_pkg: Optional[Package] = self._registry_get_by_name(name=name, version=version)
                assert dep_pkg is not None, f"Failed to load {package.name} (unknown dependency: {name}/{version})"
                self._load_q.put((dep_pkg, self._download_package(dep_pkg)))

    def _find_nupm_package(self, name: str, version: Optional[str]) -> Optional[Package]:
        for registry in self._registries:
            if isinstance(registry, NupmPackageRegistry):
                if (p := registry.get_by_name(name=name, version=version)) is not None:
                    return p
        return None

    def _generate_nu_plugins(self) -> None:
        ls_plugins_proc = subprocess.run(
            ["nu", "--commands", "plugin list | to json"],
            stdout=subprocess.PIPE,
        )
        assert ls_plugins_proc.returncode == 0, "Failed to list currently installed plugins."
        ls_plugins = json.loads(ls_plugins_proc.stdout)
        assert isinstance(ls_plugins, list), "Nushell changed its `plugin list` output format"
        for rm_plugin in (plugin["name"] for plugin in ls_plugins if (
            plugin["filename"].startswith(BASEDIRECTORY)  # ignore non numng plugins
            and plugin["filename"] not in self._nu_plugin_paths
        )):
            logger.debug(f"remove nu plugin: {rm_plugin}")
            rm_plugin_proc = subprocess.run(
                ["nu", "--commands", f"plugin rm {json.dumps(rm_plugin)}"],
                stdout=subprocess.DEVNULL,
            )
            assert rm_plugin_proc.returncode == 0, f"Failed to remove plugin {rm_plugin} due to a nushell error (did the commands change?)"
        for add_plugin in (plugin_path for plugin_path in self._nu_plugin_paths if (
            not any(True for i in ls_plugins if i["filepath"] == plugin_path)
        )):
            logger.debug(f"add nu plugin: {add_plugin}")
            add_plugin_proc = subprocess.run(
                ["nu", "--commands", f"plugin add {json.dumps(add_plugin)}"],
                stdout=subprocess.DEVNULL,
            )
            assert add_plugin_proc.returncode == 0, f"Failed to add plugin {add_plugin} due to a nushell error (did the commands change?)"


def load_package_from_json(
    json_data: Dict[str, Any],
    allow_no_name: bool = False,
) -> Package:
    if isinstance(json_data, str):
        json_data = {"name": json_data}
    assert allow_no_name or "name" in json_data, f"Unable to load package without name ({json.dumps(json_data)})"
    result: Package = Package(
        name=json_data.get("name") or "NO_NAME_PACKAGE",
        source_type=json_data.get("source_type", None),
        source_uri=json_data.get("source_uri", None),
        source_git_ref=json_data.get("git_ref", None),
        source_path_offset=json_data.get("path_offset", None),
        depends=([] if "depends" in json_data else None),
        registries=[load_package_from_json(dep, allow_no_name=True) for dep in _listify(json_data.get("registry"))] or None,
        package_format=json_data.get("package_format", None),
        extra_data=(tmp if (tmp := {k: v for k, v in json_data.items() if k not in (
            "name", "source_type", "source_uri", "git_ref", "path_offset", "depends", "registry",
            "package_format",
        )}) != {} else None),
    )
    for dependency in _listify(json_data.get("depends")):
        assert isinstance(result.depends, list)  # linter-fix (its impossible)
        result.depends.append(load_package_from_json(dependency))
    return result


def get_git_ref_path(url: str, ref: Optional[str] = None, download: bool = False, update: bool = False) -> str:
    ref = ref or "main"
    assert "://" in url, f"Invalid git url (missing ://): {url}"
    base_path = path.join(
        BASEDIRECTORY,
        "store", "git",
        *(filesystem_safe(i) for i in url.split("://", 1)[1].split("/")),
    )
    bare_path = path.join(base_path, "__bare__")
    ref_path = path.join(base_path, filesystem_safe(ref))
    
    if not download:
        return path.join(base_path, ref)
    logger.debug(f"git downloading {url}")

    if not path.exists(bare_path):
        logger.debug("clone bare")
        makedirs(base_path, exist_ok=True)
        clone_result = subprocess.run(
            ["git", "clone", "--bare", "--quiet", "--depth=1", url, "__bare__"],
            # shallow copies improve speed and if handled correct work identically
            cwd=base_path,
            stdout=subprocess.DEVNULL,
        )
        assert clone_result.returncode == 0, f"Failed to git clone {url}"

    if not path.exists(ref_path):
        logger.debug(f"fetch {url} {ref}")
        fetch_result = subprocess.run(
            ["git", "fetch", "--quiet", "--depth=1", "--tags", "origin", ref],
            # without "--tags" we have to manually figure out weather its a branch, tag, commit, or whatever and specify it, since git otherwise just dumps it into FETCH_HEAD without storing it.
            # "--tags" fetches all tags, but well at least its shallow..
            cwd=bare_path,
            stdout=subprocess.DEVNULL,
        )
        if fetch_result.returncode != 0:
            logger.debug(f"fetch failed for {url} {ref}")
            if all(i in "0123456789abcdef" for i in ref):
                logger.debug("attempting to fix potential short-hash problem via unshallow")
                fetch_result = subprocess.run(["git", "fetch", "--unshallow", "--quiet"], cwd=bare_path, stdout=subprocess.DEVNULL)
        logger.debug("worktree add")
        worktree_result = subprocess.run(["git", "worktree", "add", "--quiet", ref_path, ref], cwd=bare_path, stdout=subprocess.DEVNULL)
        if worktree_result.returncode != 0:
            # git_ref is most likely a branch <https://stackoverflow.com/questions/55909951> -> try to add as local branch <https://stackoverflow.com/questions/11266478>
            logger.debug("attempting to fix potential git-branch problem via second fetch")
            fetch_result = subprocess.run(
                ["git", "fetch", "--quiet", "--depth=1", "--tags", "origin", f"{ref}:{ref}"],
                cwd=bare_path,
                stdout=subprocess.DEVNULL,
            )
            assert fetch_result.returncode == 0, f"Failed to fetch git_ref {ref} of {url} as a branch"
            # retry worktree
            worktree_result = subprocess.run(["git", "worktree", "add", "--quiet", ref_path, ref], cwd=bare_path, stdout=subprocess.DEVNULL)
            assert worktree_result.returncode == 0, f"Failed to add a git worktree for {ref} of {url}"
    elif update:
        logger.debug("update")
        subprocess.run(["git", "clean", "-qfdx", "-e", "/target"], cwd=ref_path, stdout=subprocess.DEVNULL)
        # "-e /release" keeps the `cargo` cache improving (re-)build speed
        r = subprocess.run(["git", "fetch", "--quiet", "origin", ref], cwd=ref_path, stdout=subprocess.DEVNULL)
        assert r.returncode == 0, f"Failed to fetch update {url} {ref}"
        r = subprocess.run(["git", "reset", "--hard", "--quiet", f"FETCH_HEAD"], cwd=ref_path, stdout=subprocess.DEVNULL)
        assert r.returncode == 0, f"Failed to reset to update {url} {ref}"

    return path.join(base_path, ref)


def filesystem_safe(text: str) -> str:
    return "".join((i if i in VALID_FILESYSTEM_CHARACTERS else "_" for i in text))


def download_file(url: str, local_file: str) -> None:
    import requests
    with requests.get(url, stream=True) as r:
        r.raise_for_status()
        with open(local_file, "wb") as fp:
            for chunk in r.iter_content(chunk_size=8192):
                fp.write(chunk)


def load_nuon(text: str) -> Any:
    # nope im not writing a nuon parser for this - not interrested in "13kb" or whatever
    to_json_proc = subprocess.Popen(
        ["nu", "--no-config-file", "--stdin", "--commands", "$in | from nuon | to json"],
        stdout=subprocess.PIPE,
        stdin=subprocess.PIPE,
    )
    json_bytes: bytes = to_json_proc.communicate(input=text.encode(encoding="utf-8", errors="replace"))[0]
    assert to_json_proc.wait() == 0, 'Failed to convert nuon to json using subshell: ' + text.replace("\n", "\\n")
    return json.loads(json_bytes.decode(encoding="utf-8"))


def main() -> None:
    import argparse
    parser = argparse.ArgumentParser(
        prog="numng",
        description="NUshell MaNaGer: manage packages and more within nushell",
    )
    parser.add_argument("-n", "--nu-config", action="store_true", help="Shortcut to target the shell-config")
    parser.add_argument("-p", "--package-file", help="The target package file")
    parser.add_argument("-v", "--verbose", action="store_true", help="More verbose logging")
    subparsers = parser.add_subparsers(dest="cmd", required=True)

    parser_build = subparsers.add_parser("build", aliases=["b"], help="Build the package")
    parser_build.add_argument("--nupm-home", help="Nupm home directory")
    parser_build.add_argument("-o", "--overlay-file", help="Generate a overlay file at path")
    parser_build.add_argument("-s", "--script-file", help="Generate a script file for `source` loading at path")
    parser_build.add_argument("-u", "--pull-updates", action="store_true", help="Pull updates for already installed packages")
    parser_build.add_argument("-b", "--allow-build-commands", choices=["true", "false"], help="Ignore `build_command`s in packages (overrides the setting in the base-package)")

    parser_build = subparsers.add_parser("init", aliases=["i"], help="Initialize a new package in the current directory (or shell-config in its directory)")

    args = parser.parse_args()
    if args.verbose:
        log_handler.setLevel(logging.DEBUG)
    # assumption: nu-config in ~/.config/nushell: https://github.com/nushell/nushell/discussions/9019
    nu_config_subdir: str = path.abspath(path.join(path.expanduser("~"), ".config", "nushell", "numng"))
    package_file: Optional[str] = path.abspath(args.package_file) if args.package_file is not None else None
    if package_file is None and args.nu_config:
        package_file = path.join(nu_config_subdir, "numng.json")
    if package_file is None and path.exists("numng.json"):
        package_file = path.abspath("numng.json")

    if args.cmd in ("build", "b"):
        if package_file is None:
            logger.warning("No package file specified. Use --package-file FILEPATH or --nu-config.")
            return
        nupm_home: Optional[str] = args.nupm_home
        if nupm_home is None and args.nu_config:
            nupm_home = path.join(BASEDIRECTORY, "nu_config_nupm_home")
        # if nupm_home is None and not args.no_auto_nupm_home:
        #     nupm_home = path.abspath("numng_nupm_home")
        script_file: Optional[str] = args.script_file
        if script_file is None and args.nu_config:
            script_file = path.join(nu_config_subdir, "load_script.nu")
        try:
            Loader(
                package_file,
                generate_script=script_file,
                generate_overlay=args.overlay_file,
                nupm_home=nupm_home,
                delete_existing_nupm_home=True,
                pull_updates=args.pull_updates,
                handle_nu_plugins=args.nu_config,
                allow_build_commands=args.allow_build_commands,
            )
        except AssertionError as exc:
            if exc.args:
                logger.error(exc.args[0])
            else:
                raise exc
        return

    if args.cmd in ("init", "i"):
        dir: str = nu_config_subdir if args.nu_config else path.curdir
        if args.nu_config and not path.exists(dir):
            makedirs(dir)
        if not path.exists(numng_json := path.join(dir, "numng.json")):
            with open(numng_json, "w") as fp:
                json.dump({
                    "name": "nu-config" if args.nu_config else path.split(path.abspath(dir))[1],
                    **({"depends": [{"name": "jan9103/numng"}]} if args.nu_config else {}),
                    "registry": [{"source_uri": "https://github.com/Jan9103/numng_repo", "package_format": "numng", "path_offset": "repo"}],
                }, fp, indent=4)
        if args.nu_config and not path.exists(ls := path.join(dir, "load_script.nu")):
            nupm_home = path.join(BASEDIRECTORY, "nu_config_nupm_home")
            with open(ls, "w") as fp:
                fp.write("")
        if args.nu_config:
            nu_configfile: Optional[str] = next((i for i in [
                path.join(environ.get("XDG_CONFIG_HOME", "") or path.join(path.expanduser("~"), ".config"), "nushell", "config.nu"),
                path.join(path.expanduser("~"), ".config", "nushell", "config.nu"),
                path.join(path.expanduser("~"), "Library", "Application Support", "nushell", "config.nu"),
            ] if path.exists(i)), None)
            if (not nu_configfile) or (input("Append the load-line to your nu-config automatically? (yes or no): ").lower() != "yes"):
                print(f"Please add `source {path.join(dir, 'load_script.nu')}` to the `$nu.config-path` file")
            else:
                with open(nu_configfile, "a") as fp:
                    fp.write(f"\n\n# Load numng\nsource {path.join(dir, 'load_script.nu')}")
            if input(f"Run numng build command now?\n({orig_argv[0]} {orig_argv[1]} --nu-config build)\n(yes or no): ") == "yes":
                subprocess.run([orig_argv[0], orig_argv[1], "--nu-config", "build"])


if __name__ == "__main__":
    main()
