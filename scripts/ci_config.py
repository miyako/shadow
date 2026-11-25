#!/usr/bin/env python3
"""ci_config.py — CI configuration and resource assembly for shadow component.

Subcommands:
    assemble  — Copy CLI binaries to component Resources/bin/
    verify    — Verify assembled resources have correct SHA256 manifests
    check     — Verify component structure integrity
"""

import argparse
import hashlib
import json
import os
import shutil
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
COMPONENT_RESOURCES = os.path.join(REPO_ROOT, "component", "shadow", "Resources")
CLI_TARGET_DIR = os.path.join(REPO_ROOT, "cli", "target")


def compute_sha256(filepath):
    h = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def cmd_assemble(args):
    """Copy CLI binaries to Resources/bin/ with manifest files."""
    print("=== Assembling resources ===")
    
    bin_dir = os.path.join(COMPONENT_RESOURCES, "bin")
    os.makedirs(bin_dir, exist_ok=True)
    
    assembled = []
    
    # macOS universal
    universal_path = os.path.join(CLI_TARGET_DIR, "universal", "shadow-cli")
    if os.path.isfile(universal_path):
        dest = os.path.join(bin_dir, "shadow-cli")
        shutil.copy2(universal_path, dest)
        os.chmod(dest, 0o755)
        manifest = {
            "target": "macos-universal",
            "binary": "shadow-cli",
            "sha256": compute_sha256(dest),
            "size_bytes": os.path.getsize(dest),
        }
        with open(dest + ".manifest.json", "w") as f:
            json.dump(manifest, f, indent=2)
        print(f"  [OK] macOS universal → {dest}")
        assembled.append(dest)
    else:
        print(f"  [SKIP] macOS universal binary not found at {universal_path}")
    
    # Windows x64
    windows_path = os.path.join(CLI_TARGET_DIR, "x86_64-pc-windows-gnu", "release", "shadow-cli.exe")
    if not os.path.isfile(windows_path):
        windows_path = os.path.join(CLI_TARGET_DIR, "release", "shadow-cli.exe")
    if os.path.isfile(windows_path):
        dest = os.path.join(bin_dir, "shadow-cli.exe")
        shutil.copy2(windows_path, dest)
        manifest = {
            "target": "windows-x64",
            "binary": "shadow-cli.exe",
            "sha256": compute_sha256(dest),
            "size_bytes": os.path.getsize(dest),
        }
        with open(dest + ".manifest.json", "w") as f:
            json.dump(manifest, f, indent=2)
        print(f"  [OK] Windows x64 → {dest}")
        assembled.append(dest)
    else:
        print(f"  [SKIP] Windows binary not found")
    
    # Copy licenses
    licenses_src = os.path.join(COMPONENT_RESOURCES, "licenses", "LICENSE.txt")
    if os.path.isfile(licenses_src):
        print(f"  [OK] Licenses present")
    
    if assembled:
        print(f"\n  Assembled {len(assembled)} binaries")
    else:
        print(f"\n  WARNING: No binaries assembled")
    
    return len(assembled) > 0


def cmd_verify(args):
    """Verify SHA256 manifests match."""
    print("=== Verifying resources ===")
    
    bin_dir = os.path.join(COMPONENT_RESOURCES, "bin")
    if not os.path.isdir(bin_dir):
        print("  [FAIL] Resources/bin not found")
        return False
    
    all_ok = True
    for fname in os.listdir(bin_dir):
        if fname.endswith(".manifest.json"):
            manifest_path = os.path.join(bin_dir, fname)
            with open(manifest_path) as f:
                manifest = json.load(f)
            
            binary_path = os.path.join(bin_dir, manifest["binary"])
            if not os.path.isfile(binary_path):
                print(f"  [FAIL] Binary missing: {manifest['binary']}")
                all_ok = False
                continue
            
            actual_sha = compute_sha256(binary_path)
            if actual_sha != manifest["sha256"]:
                print(f"  [FAIL] SHA256 mismatch for {manifest['binary']}")
                all_ok = False
            else:
                print(f"  [OK] {manifest['target']}: SHA256 verified")
    
    return all_ok


def cmd_check(args):
    """Check component structure integrity."""
    print("=== Checking component structure ===")
    
    component = os.path.join(REPO_ROOT, "component", "shadow")
    expected = [
        "Project/shadow.4DProject",
        "Project/Sources/Classes/Generator.4dm",
        "Project/Sources/Classes/_CLI.4dm",
        "Project/Sources/Classes/_Process.4dm",
        "Project/Sources/Methods/_CaptureError.4dm",
        "Project/Sources/dependencies.json",
        "Documentation/Classes/Generator.md",
        "Resources/en.lproj/syntaxEN.json",
    ]
    
    all_ok = True
    for f in expected:
        path = os.path.join(component, f)
        if os.path.isfile(path):
            print(f"  [OK] {f}")
        else:
            print(f"  [MISSING] {f}")
            all_ok = False
    
    return all_ok


def main():
    parser = argparse.ArgumentParser(description="CI configuration for shadow component")
    sub = parser.add_subparsers(dest="command")
    
    sub.add_parser("assemble", help="Copy CLI binaries to Resources")
    sub.add_parser("verify", help="Verify SHA256 manifests")
    sub.add_parser("check", help="Check component structure")
    
    args = parser.parse_args()
    
    if args.command == "assemble":
        ok = cmd_assemble(args)
    elif args.command == "verify":
        ok = cmd_verify(args)
    elif args.command == "check":
        ok = cmd_check(args)
    else:
        parser.print_help()
        sys.exit(1)
    
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
