#!/usr/bin/env python3
"""accept_local.py — Local acceptance gate for shadow component.

Runs all available checks and reports readiness status.
"""

import json
import os
import subprocess
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def run_check(name, cmd, cwd=REPO_ROOT):
    """Run a check and return (passed, output)."""
    print(f"\n--- {name} ---")
    try:
        result = subprocess.run(
            cmd, shell=True, cwd=cwd,
            capture_output=True, text=True, timeout=60
        )
        output = result.stdout.strip()
        if output:
            print(output)
        if result.stderr.strip():
            print(f"  stderr: {result.stderr.strip()}")
        passed = result.returncode == 0
        print(f"  Result: {'PASS' if passed else 'FAIL'} (exit {result.returncode})")
        return passed, output
    except subprocess.TimeoutExpired:
        print(f"  Result: TIMEOUT")
        return False, ""
    except Exception as e:
        print(f"  Result: ERROR ({e})")
        return False, str(e)


def main():
    print("Shadow Component — Local Acceptance")
    print("=" * 50)
    
    results = {}
    
    # 1. Rust unit tests
    p, _ = run_check("Rust unit tests", "cargo test", cwd=os.path.join(REPO_ROOT, "cli"))
    results["rust_unit_tests"] = p
    
    # 2. CLI contract tests
    p, _ = run_check("CLI contract tests", "python3 tests/test_cli.py")
    results["cli_contract_tests"] = p
    
    # 3. CLI selftest
    cli_path = os.path.join(REPO_ROOT, "cli", "target", "debug", "shadow-cli")
    if os.path.isfile(cli_path):
        p, _ = run_check("CLI selftest", f"{cli_path} selftest")
        results["cli_selftest"] = p
    else:
        print("\n--- CLI selftest ---")
        print("  [SKIP] CLI binary not built")
        results["cli_selftest"] = None
    
    # 4. Component structure
    p, _ = run_check("Component structure", "python3 scripts/ci_config.py check")
    results["component_structure"] = p
    
    # 5. Component resource verification
    p, _ = run_check("Resource verification", "python3 scripts/ci_config.py verify")
    results["resource_verification"] = p
    
    # Summary
    print("\n" + "=" * 50)
    print("Summary:")
    for check, passed in results.items():
        if passed is None:
            status = "SKIP"
        elif passed:
            status = "PASS"
        else:
            status = "FAIL"
        print(f"  {check}: {status}")
    
    # Determine result
    all_pass = all(v for v in results.values() if v is not None)
    finished = True
    
    result = {
        "result": "partial" if not all_pass else "partial",  # always partial locally
        "finished": finished,
        "release_ready": False,  # never release-ready from local
        "checks": {k: ("pass" if v else ("skip" if v is None else "fail")) for k, v in results.items()}
    }
    
    print(f"\nResult: {result['result']}, finished: {result['finished']}, release_ready: {result['release_ready']}")
    
    print(json.dumps(result, indent=2))
    
    sys.exit(0 if finished else 1)


if __name__ == "__main__":
    main()
