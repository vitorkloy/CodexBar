#!/usr/bin/env python3
"""Package the Windows CLI and Qt tray into a zip without publishing a release."""
import argparse
import os
from pathlib import Path
import re
import zipfile

REPO = Path(__file__).resolve().parents[2]


def add_file(archive, source, arcname):
    info = zipfile.ZipInfo(arcname.replace("\\", "/"))
    info.date_time = (2026, 1, 1, 0, 0, 0)
    info.external_attr = 0o644 << 16
    with open(source, "rb") as handle:
        archive.writestr(info, handle.read())


def add_directory(archive, source, arcname):
    for root, _, files in os.walk(source):
        for name in files:
            full = Path(root) / name
            relative = full.relative_to(source)
            add_file(archive, full, f"{arcname}/{relative.as_posix()}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--desktop", type=Path, required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", type=Path, default=REPO / ".local/packages")
    parser.add_argument("--bundle", type=Path, default=None)
    parser.add_argument("--deploy-dir", type=Path, default=None,
                        help="Optional windeployqt output directory copied beside the tray app")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9][a-zA-Z0-9._-]{0,79}", args.version):
        parser.error("Version must start with a digit")
    if not args.cli.is_file():
        parser.error("CLI binary is missing")
    if not args.desktop.is_file():
        parser.error("Desktop binary is missing")

    name = f"CodexBar-v{args.version}-windows-x64"
    args.output.mkdir(parents=True, exist_ok=True)
    destination = args.output / f"{name}.zip"
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        add_file(archive, args.cli, f"{name}/codexbar.exe")
        add_file(archive, args.desktop, f"{name}/codexbar-windows.exe")
        add_file(archive, REPO / "Integrations/Windows/install.ps1", f"{name}/install.ps1")
        add_file(archive, REPO / "Integrations/Windows/README.md", f"{name}/README.md")
        add_file(archive, REPO / "docs/windows.md", f"{name}/docs/windows.md")
        add_file(archive, REPO / "LICENSE", f"{name}/LICENSE")
        version_info = zipfile.ZipInfo(f"{name}/VERSION")
        version_info.date_time = (2026, 1, 1, 0, 0, 0)
        archive.writestr(version_info, f"{args.version}\n")
        if args.bundle and args.bundle.exists():
            add_directory(archive, args.bundle, f"{name}/{args.bundle.name}")
        if args.deploy_dir and args.deploy_dir.exists():
            skip = {"codexbar-windows.exe", "codexbar.exe", "Makefile", "Makefile.Debug", "Makefile.Release"}
            for path in args.deploy_dir.rglob("*"):
                if not path.is_file():
                    continue
                relative = path.relative_to(args.deploy_dir).as_posix()
                if path.name in skip or relative.startswith("debug/") or relative.endswith(".obj"):
                    continue
                add_file(archive, path, f"{name}/{relative}")
    print(destination)


if __name__ == "__main__":
    main()
