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
            System.err.println("Commands: extract-symbols");
            System.exit(1);
        }

        String command = args[0];

        try (BufferedReader reader = new BufferedReader(new InputStreamReader(System.in))) {
            String source = reader.lines().collect(Collectors.joining("\n"));

            switch (command) {
                case "extract-symbols":
                    extractSymbols(source);
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
}