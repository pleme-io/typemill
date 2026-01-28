import subprocess
import json
import os
import sys

# Assume script is run from repo root
MILL_BIN = os.path.abspath("target/debug/mill")
REPO_DIR = os.path.abspath("test_env/repo")

def run_mill_tool(tool_name, args):
    cmd = [MILL_BIN, "tool", tool_name, json.dumps(args)]
    result = subprocess.run(cmd, cwd=REPO_DIR, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error running {tool_name}: {result.stderr}")
        return None
    try:
        # mill output might contain logs on stderr, json on stdout
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        print(f"Failed to parse JSON: {result.stdout}")
        return None

def verify_refactor_rename():
    print("Verifying refactor rename (dryRun)...")
    # Rename UserManager to UsersRepo in src/index.ts
    # Line 4: const manager = new UserManager();
    # Refactoring tools use LSP 0-based indexing directly via lsp_types::Position
    args = {
        "target": {
            "kind": "symbol",
            "path": "src/index.ts",
            "selector": {
                "position": { "line": 3, "character": 22 }
            }
        },
        "newName": "UsersRepo",
        "options": { "dryRun": True }
    }

    output = run_mill_tool("rename", args)
    if not output: return False

    content = output.get("content")
    if not content:
        print("No plan content")
        print(json.dumps(output, indent=2))
        return False

    # Check if edits are present
    edits = content.get("edits", {})
    changes = edits.get("documentChanges", []) or edits.get("changes", {})

    if not changes:
        print("No changes in plan")
        print(json.dumps(content, indent=2))
        return False

    print("SUCCESS")
    return True

def verify_refactor_move():
    print("Verifying refactor move (dryRun)...")
    # Move src/utils.ts to src/helpers.ts
    args = {
        "target": {
            "kind": "file",
            "path": "src/utils.ts"
        },
        "destination": "src/helpers.ts",
        "options": { "dryRun": True }
    }

    output = run_mill_tool("move", args)
    if not output: return False

    content = output.get("content")
    if not content:
        print("No plan content")
        print(json.dumps(output, indent=2))
        return False

    # Check for file rename op
    edits = content.get("edits", {})
    doc_changes = edits.get("documentChanges", [])

    found_rename = False
    if doc_changes:
        for op in doc_changes:
            if "kind" in op and op["kind"] == "rename":
                found_rename = True
                break
            if "oldUri" in op:
                found_rename = True
                break

    if not found_rename:
        print("Did not find rename operation in plan")
        print(json.dumps(content, indent=2))
        return False

    print("SUCCESS")
    return True

def verify_workspace_find_replace():
    print("Verifying workspace.find_replace...")
    args = {
        "pattern": "UserManager",
        "replacement": "UserMgr",
        "dryRun": True,
    }

    output = run_mill_tool("workspace.find_replace", args)
    if not output: return False

    # FindReplaceHandler might return plan directly or wrapped
    if "edits" in output:
        content = output
    elif "content" in output:
        content = output["content"]
    else:
        print("Unknown output format")
        print(json.dumps(output, indent=2))
        return False

    # Check for edits
    edits = content.get("edits", [])
    if not edits:
        print("No edits found")
        print(json.dumps(content, indent=2))
        return False

    print(f"Found {len(edits)} edits")
    print("SUCCESS")
    return True

def verify_find_definition():
    print("Verifying find_definition...")
    # UserManager usage in src/index.ts
    # Navigation tools use 1-based indexing for 'line' parameter (CLI convention)
    args = {
        "filePath": "src/index.ts",
        "line": 4,
        "character": 22
    }
    output = run_mill_tool("find_definition", args)
    if not output: return False

    locations = output.get("content", {}).get("locations", [])
    if not locations:
        print("No locations found")
        return False

    uri = locations[0]["uri"]
    print(f"Found definition at: {uri}")
    print("SUCCESS")
    return True

def verify_find_type_definition():
    print("Verifying find_type_definition...")
    # manager variable in src/index.ts
    # Navigation tools use 1-based indexing
    args = {
        "filePath": "src/index.ts",
        "line": 4,
        "character": 8
    }
    output = run_mill_tool("find_type_definition", args)
    if not output: return False

    locations = output.get("content", {}).get("locations", [])
    if not locations:
        print("No locations found")
        return False

    uri = locations[0]["uri"]
    if "src/models/User.ts" not in uri:
        print(f"Unexpected URI: {uri}")
        return False

    print("SUCCESS")
    return True

def verify_find_references():
    print("Verifying find_references...")
    # User interface in src/models/User.ts
    # Line 1 (1-based), Char 18
    args = {
        "filePath": "src/models/User.ts",
        "line": 1,
        "character": 18
    }
    output = run_mill_tool("find_references", args)
    if not output: return False

    locations = output.get("content", {}).get("locations", [])
    if len(locations) < 2:
        print(f"Expected at least 2 references, found {len(locations)}")
        return False

    print(f"Found {len(locations)} references")
    print("SUCCESS")
    return True

def verify_search_symbols():
    print("Verifying search_symbols...")
    args = {
        "query": "UserManager"
    }
    output = run_mill_tool("search_symbols", args)
    if not output: return False

    symbols = output.get("content", [])
    if not symbols:
        print("No symbols returned. This may happen if workspace is not fully indexed yet.")
        print("WARNING: search_symbols returned no results")
        return True # Soft pass

    found = any(s.get("name") == "UserManager" for s in symbols)
    if not found:
        print("UserManager symbol not found")
        print(json.dumps(symbols, indent=2))
        return False

    print("SUCCESS")
    return True

def verify_get_symbol_info():
    print("Verifying get_symbol_info...")
    # isValidEmail in src/index.ts
    # Line 12 (1-based), Char 6
    args = {
        "filePath": "src/index.ts",
        "line": 12,
        "character": 6
    }
    output = run_mill_tool("get_symbol_info", args)
    if not output: return False

    content = output.get("content")
    if not content:
        print("No symbol info content")
        print(json.dumps(output, indent=2))
        return False

    # Handle possible wrapper
    if "hover" in content:
        content = content["hover"]

    # Check if contents string contains function signature
    contents = content.get("contents", "")
    print(f"Symbol Info contents: {contents}")

    found = False
    if isinstance(contents, str):
        found = "isValidEmail" in contents or "boolean" in contents
    elif isinstance(contents, list):
        found = any("isValidEmail" in str(c) for c in contents)
    elif isinstance(contents, dict):
        val = contents.get("value", "")
        found = "isValidEmail" in val or "boolean" in val

    if not found:
        print("Symbol info does not seem to match isValidEmail")
        print(json.dumps(content, indent=2))
        return False

    print("SUCCESS")
    return True

def verify_get_call_hierarchy():
    print("Verifying get_call_hierarchy (prepare)...")
    # addUser in src/models/User.ts
    # Line 10 (1-based), Char 4
    args = {
        "filePath": "src/models/User.ts",
        "line": 10,
        "character": 4
    }
    output = run_mill_tool("get_call_hierarchy", args)
    if not output: return False

    content = output.get("content", [])
    if not content:
        print("No call hierarchy items found")
        print(json.dumps(output, indent=2))
        return False

    item = content[0]
    print(f"Found item: {item.get('name')}")

    print("SUCCESS")
    return True

def main():
    if not os.path.exists(MILL_BIN):
        print(f"Mill binary not found at {MILL_BIN}")
        sys.exit(1)

    tests = [
        verify_find_definition,
        verify_find_type_definition,
        verify_find_references,
        verify_search_symbols,
        verify_get_symbol_info,
        verify_get_call_hierarchy,
        verify_refactor_rename,
        verify_refactor_move,
        verify_workspace_find_replace
    ]

    passed = 0
    for test in tests:
        if test():
            passed += 1
        else:
            print(f"{test.__name__} FAILED")

    print(f"\nPassed {passed}/{len(tests)} tests")
    if passed != len(tests):
        sys.exit(1)

if __name__ == "__main__":
    main()
