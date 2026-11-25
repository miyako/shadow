#!/usr/bin/env python3
"""build_cli.py — Build shadow-cli for all targets and create universal macOS binary.

Usage:
    python3 scripts/build_cli.py [--target TARGET] [--release]

Targets:
    macos-arm64    Build for macOS ARM64
    macos-x64      Build for macOS x86_64
    macos-universal Create universal binary via lipo
    windows-x64    Build for Windows x64 (requires cross-compilation toolchain)
    all            Build all targets (default)
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI_DIR = os.path.join(REPO_ROOT, "cli")
TARGET_DIR = os.path.join(CLI_DIR, "target")
RESOURCES_BIN = os.path.join(REPO_ROOT, "component", "shadow", "Resources", "bin")


def run(cmd, cwd=None):
    print(f"  $ {cmd}")
    result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  ERROR: {result.stderr}")
        sys.exit(1)
    if result.stdout.strip():
        print(f"  {result.stdout.strip()}")
    return result


def build_macos(arch, release=True):
    """Build for a specific macOS architecture."""
    profile = "release" if release else "debug"
    rust_target = f"{arch}-apple-darwin"
    
    print(f"\n=== Building {rust_target} ({profile}) ===")
    
    # Add target if not present
    run(f"rustup target add {rust_target}", cwd=CLI_DIR)
    
    # Build
    flags = f"--target {rust_target} --profile {profile}"
    if release:
        flags = f"--target {rust_target} --release"
    run(f"cargo build {flags}", cwd=CLI_DIR)
    
    # Locate binary
    binary_path = os.path.join(TARGET_DIR, rust_target, profile, "shadow-cli")
    if not os.path.isfile(binary_path):
        print(f"  ERROR: binary not found at {binary_path}")
        sys.exit(1)
    
    print(f"  Built: {binary_path}")
    return binary_path


def create_universal(arm64_bin, x64_bin, output_path):
    """Create universal binary using lipo."""
    print(f"\n=== Creating universal binary ===")
    
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    
    # lipo -create
    run(f"lipo -create -output {output_path} {arm64_bin} {x64_bin}")
    
    # Verify architectures
    result = run(f"lipo -archs {output_path}")
    archs = result.stdout.strip()
    if "arm64" not in archs or "x86_64" not in archs:
        print(f"  ERROR: universal binary missing architectures: {archs}")
        sys.exit(1)
    
    print(f"  Universal binary: {output_path}")
    print(f"  Architectures: {archs}")
    
    return output_path


def compute_sha256(filepath):
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def write_manifest(binary_path, target_name, profile="release"):
    """Write a build manifest with SHA256 next to the binary."""
    manifest = {
        "target": target_name,
        "binary": os.path.basename(binary_path),
        "sha256": compute_sha256(binary_path),
        "size_bytes": os.path.getsize(binary_path),
        "profile": profile,
    }
    manifest_path = binary_path + ".manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"  Manifest: {manifest_path}")
    return manifest


def build_windows_x64(release=True):
    """Build for Windows x64 (requires cross-compilation)."""
    profile = "release" if release else "debug"
    rust_target = "x86_64-pc-windows-gnu"
    
    print(f"\n=== Building {rust_target} ({profile}) ===")
    
    try:
        run(f"rustup target add {rust_target}", cwd=CLI_DIR)
        flags = f"--target {rust_target}"
        if release:
            flags += " --release"
        run(f"cargo build {flags}", cwd=CLI_DIR)
        
        binary_path = os.path.join(TARGET_DIR, rust_target, profile, "shadow-cli.exe")
        if os.path.isfile(binary_path):
            print(f"  Built: {binary_path}")
            return binary_path
    except SystemExit:
        pass
    
    print(f"  WARNING: Windows cross-compilation not available on this platform")
    print(f"  Windows build must be done on a Windows machine or CI runner")
    return None


def assemble_resources(universal_path=None, windows_path=None):
    """Copy binaries to component Resources/bin/."""
    print(f"\n=== Assembling resources ===")
    
    os.makedirs(RESOURCES_BIN, exist_ok=True)
    
    if universal_path and os.path.isfile(universal_path):
        dest = os.path.join(RESOURCES_BIN, "shadow-cli")
        shutil.copy2(universal_path, dest)
        os.chmod(dest, 0o755)
        print(f"  Copied macOS universal: {dest}")
        write_manifest(dest, "macos-universal")
    
    if windows_path and os.path.isfile(windows_path):
        dest = os.path.join(RESOURCES_BIN, "shadow-cli.exe")
        shutil.copy2(windows_path, dest)
        print(f"  Copied Windows x64: {dest}")
        write_manifest(dest, "windows-x64")


def main():
    parser = argparse.ArgumentParser(description="Build shadow-cli for all targets")
    parser.add_argument("--target", default="all",
                        choices=["all", "macos-arm64", "macos-x64", "macos-universal", "windows-x64"])
    parser.add_argument("--release", action="store_true", default=True)
    parser.add_argument("--debug", action="store_true", help="Build in debug mode")
    parser.add_argument("--assemble", action="store_true", default=True,
                        help="Assemble binaries to component Resources")
    args = parser.parse_args()
    
    release = not args.debug
    target = args.target
    
    if target in ("macos-arm64", "all"):
        arm64 = build_macos("aarch64", release)
    if target in ("macos-x64", "all"):
        x64 = build_macos("x86_64", release)
    
    universal_path = None
    if target in ("macos-universal", "all"):
        if target == "macos-universal":
            arm64 = build_macos("aarch64", release)
            x64 = build_macos("x86_64", release)
        universal_output = os.path.join(TARGET_DIR, "universal", "shadow-cli")
        universal_path = create_universal(arm64, x64, universal_output)
    
    windows_path = None
    if target in ("windows-x64", "all"):
        windows_path = build_windows_x64(release)
    
    if args.assemble:
        assemble_resources(universal_path, windows_path)
    
    print("\n=== Build complete ===")


if __name__ == "__main__":
    main()
