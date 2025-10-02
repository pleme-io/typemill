import sys
import ast
import json

def list_functions(source_code):
    """
    Parses Python source code and returns a list of function and method names.
    """
    try:
        tree = ast.parse(source_code)
        function_names = []
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef):
                function_names.append(node.name)
        return {"status": "success", "data": function_names}
    except SyntaxError as e:
        return {
            "status": "error",
            "error": {
                "type": "SyntaxError",
                "message": e.msg,
                "lineno": e.lineno,
                "offset": e.offset,
            },
        }

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(json.dumps({"status": "error", "error": {"type": "UsageError", "message": "No command provided."}}), file=sys.stderr)
        sys.exit(1)

    command = sys.argv[1]
    source = sys.stdin.read()

    if command == "list-functions":
        result = list_functions(source)
        if result["status"] == "success":
            print(json.dumps(result["data"]))
        else:
            print(json.dumps(result["error"]), file=sys.stderr)
            sys.exit(1)
    else:
        print(json.dumps({"status": "error", "error": {"type": "UsageError", "message": f"Unknown command: {command}"}}), file=sys.stderr)
        sys.exit(1)
