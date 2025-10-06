package com.codebuddy.parser;

import com.github.javaparser.JavaParser;
import com.github.javaparser.ParseResult;
import com.github.javaparser.ast.CompilationUnit;
import com.github.javaparser.ast.body.ClassOrInterfaceDeclaration;
import com.github.javaparser.ast.body.MethodDeclaration;
import com.github.javaparser.ast.visitor.VoidVisitorAdapter;
import com.google.gson.Gson;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;
import java.util.stream.Collectors;

public class Parser {

    public static void main(String[] args) {
        if (args.length == 0) {
            System.err.println("Usage: java -jar parser.jar <command>");
            System.err.println("Commands: extract-symbols, parse-imports, add-import <path>, remove-import <path>, rewrite-imports <old> <new>, parse-package");
            System.exit(1);
        }

        String command = args[0];

        try (BufferedReader reader = new BufferedReader(new InputStreamReader(System.in))) {
            String source = reader.lines().collect(Collectors.joining("\n"));

            switch (command) {
                case "extract-symbols":
                    extractSymbols(source);
                    break;
                case "parse-imports":
                    parseImports(source);
                    break;
                case "add-import":
                    if (args.length < 2) {
                        System.err.println("add-import requires import path argument");
                        System.exit(1);
                    }
                    addImport(source, args[1]);
                    break;
                case "remove-import":
                    if (args.length < 2) {
                        System.err.println("remove-import requires import path argument");
                        System.exit(1);
                    }
                    removeImport(source, args[1]);
                    break;
                case "rewrite-imports":
                    if (args.length < 3) {
                        System.err.println("rewrite-imports requires old and new path arguments");
                        System.exit(1);
                    }
                    rewriteImports(source, args[1], args[2]);
                    break;
                case "parse-package":
                    parsePackage(source);
                    break;
                default:
                    System.err.println("Unknown command: " + command);
                    System.exit(1);
            }
        } catch (Exception e) {
            System.err.println("Error processing input: " + e.getMessage());
            System.exit(1);
        }
    }

    private static void extractSymbols(String source) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.err.println("Failed to parse Java source.");
            // Output an empty JSON array on failure
            System.out.println("[]");
            return;
        }

        CompilationUnit cu = result.getResult().get();
        List<SymbolInfo> symbols = new ArrayList<>();
        SymbolVisitor visitor = new SymbolVisitor();
        visitor.visit(cu, symbols);

        Gson gson = new Gson();
        System.out.println(gson.toJson(symbols));
    }

    private static void parseImports(String source) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.out.println("[]");
            return;
        }

        CompilationUnit cu = result.getResult().get();
        List<ImportInfo> imports = new ArrayList<>();

        cu.getImports().forEach(importDecl -> {
            imports.add(new ImportInfo(
                importDecl.getNameAsString(),
                importDecl.isStatic(),
                importDecl.isAsterisk()
            ));
        });

        Gson gson = new Gson();
        System.out.println(gson.toJson(imports));
    }

    private static void addImport(String source, String importPath) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.out.println(source);
            return;
        }

        CompilationUnit cu = result.getResult().get();

        // Check if import already exists
        boolean exists = cu.getImports().stream()
            .anyMatch(i -> i.getNameAsString().equals(importPath));

        if (!exists) {
            cu.addImport(importPath);
        }

        System.out.println(cu.toString());
    }

    private static void removeImport(String source, String importPath) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.out.println(source);
            return;
        }

        CompilationUnit cu = result.getResult().get();
        cu.getImports().removeIf(i -> i.getNameAsString().equals(importPath));

        System.out.println(cu.toString());
    }

    private static void rewriteImports(String source, String oldPath, String newPath) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.out.println(source);
            return;
        }

        CompilationUnit cu = result.getResult().get();

        cu.getImports().forEach(importDecl -> {
            String currentPath = importDecl.getNameAsString();
            if (currentPath.equals(oldPath) || currentPath.startsWith(oldPath + ".")) {
                String updatedPath = currentPath.replace(oldPath, newPath);
                importDecl.setName(updatedPath);
            }
        });

        System.out.println(cu.toString());
    }

    private static void parsePackage(String source) {
        JavaParser javaParser = new JavaParser();
        ParseResult<CompilationUnit> result = javaParser.parse(source);

        if (!result.isSuccessful() || !result.getResult().isPresent()) {
            System.out.println("null");
            return;
        }

        CompilationUnit cu = result.getResult().get();
        String packageName = cu.getPackageDeclaration()
            .map(pd -> pd.getNameAsString())
            .orElse(null);

        System.out.println(packageName != null ? packageName : "null");
    }

    private static class SymbolVisitor extends VoidVisitorAdapter<List<SymbolInfo>> {
        @Override
        public void visit(ClassOrInterfaceDeclaration n, List<SymbolInfo> arg) {
            super.visit(n, arg);
            String kind = n.isInterface() ? "Interface" : "Class";
            arg.add(new SymbolInfo(n.getNameAsString(), kind, n.getBegin().get().line));
        }

        @Override
        public void visit(MethodDeclaration n, List<SymbolInfo> arg) {
            super.visit(n, arg);
            arg.add(new SymbolInfo(n.getNameAsString(), "Method", n.getBegin().get().line));
        }
    }

    // A simple class to hold symbol information for JSON serialization
    private static class SymbolInfo {
        String name;
        String kind;
        int line;

        SymbolInfo(String name, String kind, int line) {
            this.name = name;
            this.kind = kind;
            this.line = line;
        }
    }

    // A simple class to hold import information for JSON serialization
    private static class ImportInfo {
        String path;
        boolean isStatic;
        boolean isWildcard;

        ImportInfo(String path, boolean isStatic, boolean isWildcard) {
            this.path = path;
            this.isStatic = isStatic;
            this.isWildcard = isWildcard;
        }
    }
}