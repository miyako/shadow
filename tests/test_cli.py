#!/usr/bin/env python3
"""CLI contract test runner for shadow-cli.

Reads test vectors from cli-contract.json, executes each against the
shadow-cli binary, and verifies exit codes, stdout JSON, and stderr.
"""

import json
import os
import subprocess
import sys
import tempfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
CONTRACT_FILE = os.path.join(SCRIPT_DIR, "cli-contract.json")

# Find the CLI binary
CLI_PATHS = [
    os.path.join(REPO_ROOT, "cli", "target", "debug", "shadow-cli"),
    os.path.join(REPO_ROOT, "cli", "target", "release", "shadow-cli"),
]


def find_cli():
    for p in CLI_PATHS:
        if os.path.isfile(p) and os.access(p, os.X_OK):
            return p
    # Try just "shadow-cli" from PATH
    return "shadow-cli"


def run_vector(cli_path, vector):
    """Run a single test vector and return (passed, message)."""
    vid = vector.get("id", "<unknown>")
    args = vector["args"]
    cmd = [cli_path] + args

    stdin_data = None
    if "stdin_file" in vector:
        stdin_path = os.path.join(REPO_ROOT, vector["stdin_file"])
        with open(stdin_path, "rb") as f:
            stdin_data = f.read()

    try:
        proc = subprocess.run(
            cmd,
            input=stdin_data,
            capture_output=True,
            timeout=30,
            cwd=REPO_ROOT,
        )
    except subprocess.TimeoutExpired:
        return False, f"[{vid}] TIMEOUT after 30s"
    except FileNotFoundError:
        return False, f"[{vid}] CLI binary not found: {cli_path}"

    exit_code = proc.returncode
    stdout = proc.stdout.decode("utf-8", errors="replace")
    stderr = proc.stderr.decode("utf-8", errors="replace")

    # Check exit code
    expected_exit = vector.get("exit_code", 0)
    if exit_code != expected_exit:
        return False, f"[{vid}] exit_code: expected {expected_exit}, got {exit_code}. stderr: {stderr.strip()}"

    # Check stdout JSON (exact match)
    if "stdout_json" in vector:
        try:
            actual = json.loads(stdout.strip())
        except json.JSONDecodeError as e:
            return False, f"[{vid}] stdout not valid JSON: {e}. stdout: {stdout.strip()}"
        if actual != vector["stdout_json"]:
            return False, f"[{vid}] stdout JSON mismatch.\n  expected: {vector['stdout_json']}\n  got:      {actual}"

    # Check stdout JSON (contains — partial match)
    if "stdout_json_contains" in vector:
        try:
            actual = json.loads(stdout.strip())
        except json.JSONDecodeError as e:
            return False, f"[{vid}] stdout not valid JSON: {e}. stdout: {stdout.strip()}"
        expected = vector["stdout_json_contains"]
        for key, val in expected.items():
            if key not in actual:
                return False, f"[{vid}] stdout JSON missing key: {key}"
            if isinstance(val, dict):
                for k2, v2 in val.items():
                    if k2 not in actual[key]:
                        return False, f"[{vid}] stdout JSON missing nested key: {key}.{k2}"
                    if isinstance(v2, list):
                        for item in v2:
                            if item not in actual[key][k2]:
                                return False, f"[{vid}] stdout JSON missing item in {key}.{k2}: {item}"
                    elif actual[key][k2] != v2:
                        return False, f"[{vid}] stdout JSON {key}.{k2}: expected {v2}, got {actual[key][k2]}"
            elif actual[key] != val:
                return False, f"[{vid}] stdout JSON {key}: expected {val}, got {actual[key]}"

    # Check stdout contains (plain text)
    if "stdout_contains" in vector:
        if vector["stdout_contains"] not in stdout:
            return False, f"[{vid}] stdout does not contain: '{vector['stdout_contains']}'"

    # Check stdout exact text
    if "stdout" in vector and vector["stdout"] != "":
        if stdout.strip() != vector["stdout"].strip():
            return False, f"[{vid}] stdout mismatch.\n  expected: {vector['stdout']}\n  got:      {stdout.strip()}"

    # Check stderr
    if "stderr" in vector:
        if vector["stderr"] == "" and stderr.strip() != "":
            return False, f"[{vid}] stderr should be empty, got: {stderr.strip()}"
        if vector["stderr"] != "" and vector["stderr"] not in stderr:
            return False, f"[{vid}] stderr does not contain expected text"

    # Check stderr JSON contains
    if "stderr_json_contains" in vector:
        try:
            actual_err = json.loads(stderr.strip())
        except json.JSONDecodeError as e:
            return False, f"[{vid}] stderr not valid JSON: {e}. stderr: {stderr.strip()}"
        expected_err = vector["stderr_json_contains"]
        for key, val in expected_err.items():
            if key not in actual_err:
                return False, f"[{vid}] stderr JSON missing key: {key}"
            if isinstance(val, dict):
                for k2, v2 in val.items():
                    if k2 not in actual_err[key]:
                        return False, f"[{vid}] stderr JSON missing nested key: {key}.{k2}"
                    if actual_err[key][k2] != v2:
                        return False, f"[{vid}] stderr JSON {key}.{k2}: expected {v2}, got {actual_err[key][k2]}"

    # Check output file exists
    if "expected_output_exists" in vector and vector["expected_output_exists"]:
        output_arg_idx = None
        for i, a in enumerate(args):
            if a == "--output" and i + 1 < len(args):
                output_arg_idx = i + 1
                break
        if output_arg_idx is None:
            return False, f"[{vid}] no --output argument found"
        output_path = args[output_arg_idx]
        if output_path == "-":
            return True, f"[{vid}] PASS (stdout output)"
        if not os.path.isabs(output_path):
            output_path = os.path.join(REPO_ROOT, output_path)
        if not os.path.isfile(output_path):
            return False, f"[{vid}] output file not created: {output_path}"
        # Check minimum dimensions
        if "expected_width_min" in vector:
            file_size = os.path.getsize(output_path)
            if file_size < 100:
                return False, f"[{vid}] output file too small ({file_size} bytes)"

    return True, f"[{vid}] PASS"


def main():
    if len(sys.argv) > 1:
        cli_path = sys.argv[1]
    else:
        cli_path = find_cli()

    print(f"CLI: {cli_path}")
    print()

    with open(CONTRACT_FILE) as f:
        contract = json.load(f)

    vectors = contract["vectors"]
    passed = 0
    failed = 0

    for vector in vectors:
        ok, msg = run_vector(cli_path, vector)
        print(f"  {msg}")
        if ok:
            passed += 1
        else:
            failed += 1

    print()
    print(f"Results: {passed} passed, {failed} failed, {len(vectors)} total")

    if failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
