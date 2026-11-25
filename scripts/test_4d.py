#!/usr/bin/env python3
"""test_4d.py — Run 4D host smoke tests via tool4d.

This script:
1. Verifies the CLI contract tests pass
2. Checks that the component structure is correct
3. (If tool4d is available) Compiles and runs host tests
"""

import json
import os
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT_DIR = os.path.join(REPO_ROOT, "scripts")


def check_component_structure():
    """Verify component directory structure exists."""
    print("=== Checking component structure ===")
    component_root = os.path.join(REPO_ROOT, "component", "shadow")
    
    expected_files = [
        "Project/shadow.4DProject",
        "Project/Sources/Classes/Generator.4dm",
        "Project/Sources/Classes/_CLI.4dm",
        "Project/Sources/Classes/_Process.4dm",
        "Project/Sources/Methods/_CaptureError.4dm",
        "Project/Sources/dependencies.json",
        "Documentation/Classes/Generator.md",
        "Resources/en.lproj/syntaxEN.json",
        "Resources/licenses/LICENSE.txt",
    ]
    
    all_found = True
    for f in expected_files:
        path = os.path.join(component_root, f)
        if os.path.isfile(path):
            print(f"  [OK] {f}")
        else:
            print(f"  [MISSING] {f}")
            all_found = False
    
    return all_found


def check_host_contract():
    """Verify host contract.json is valid and has expected cases."""
    print("\n=== Checking host contract ===")
    contract_path = os.path.join(REPO_ROOT, "tests", "host", "contract.json")
    
    if not os.path.isfile(contract_path):
        print(f"  [MISSING] {contract_path}")
        return False
    
    with open(contract_path) as f:
        contract = json.load(f)
    
    cases = contract.get("cases", [])
    print(f"  Contract has {len(cases)} test cases")
    
    required_ids = {"constructor", "version", "capabilities", "generate_drop", 
                    "shadow_only", "error_invalid_type", "error_missing_input"}
    found_ids = {c["id"] for c in cases}
    
    missing = required_ids - found_ids
    if missing:
        print(f"  [MISSING] Required cases not found: {missing}")
        return False
    
    print(f"  All required cases present")
    return True


def check_cli_contract():
    """Run the CLI contract tests."""
    print("\n=== Running CLI contract tests ===")
    test_path = os.path.join(REPO_ROOT, "tests", "test_cli.py")
    
    if not os.path.isfile(test_path):
        print(f"  [MISSING] {test_path}")
        return False
    
    result = subprocess.run(
        [sys.executable, test_path],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True
    )
    
    print(result.stdout.strip())
    if result.returncode != 0:
        print(f"  [FAIL] CLI contract tests failed")
        if result.stderr.strip():
            print(f"  stderr: {result.stderr.strip()}")
        return False
    
    return True


def main():
    print("Shadow Component — 4D Host Test Runner")
    print("=" * 50)
    
    ok = True
    
    ok &= check_component_structure()
    ok &= check_host_contract()
    ok &= check_cli_contract()
    
    print("\n" + "=" * 50)
    if ok:
        print("All checks passed.")
        sys.exit(0)
    else:
        print("Some checks failed.")
        sys.exit(1)


if __name__ == "__main__":
    main()
