using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace csharp_parser;

public class Program
{
    public static async Task Main(string[] args)
    {
        try
        {
            var sourceCode = await Console.In.ReadToEndAsync();
            if (string.IsNullOrWhiteSpace(sourceCode))
            {
                Console.Error.WriteLine("Error: Source code is empty or null.");
                return;
            }
            var symbols = ParseSourceCode(sourceCode);
            var json = JsonSerializer.Serialize(symbols, new JsonSerializerOptions
            {
                WriteIndented = false,
                DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
            });
            Console.WriteLine(json);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"An unexpected error occurred: {ex.Message}");
            Console.Error.WriteLine(ex.StackTrace);
        }
    }

    public static List<Symbol> ParseSourceCode(string sourceCode)
    {
        var tree = CSharpSyntaxTree.ParseText(sourceCode);
        var root = tree.GetRoot();
        var walker = new SymbolWalker();
        walker.Visit(root);
        return walker.Symbols;
    }
}

public class SymbolWalker : CSharpSyntaxWalker
{
    public List<Symbol> Symbols { get; } = new();

    public override void VisitClassDeclaration(ClassDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Class",
            Location = GetLocation(node)
        });
        base.VisitClassDeclaration(node);
    }

    public override void VisitInterfaceDeclaration(InterfaceDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Interface",
            Location = GetLocation(node)
        });
        base.VisitInterfaceDeclaration(node);
    }

    public override void VisitEnumDeclaration(EnumDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Enum",
            Location = GetLocation(node)
        });
        base.VisitEnumDeclaration(node);
    }

    public override void VisitStructDeclaration(StructDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Struct",
            Location = GetLocation(node)
        });
        base.VisitStructDeclaration(node);
    }

    public override void VisitMethodDeclaration(MethodDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Method",
            Location = GetLocation(node)
        });
        base.VisitMethodDeclaration(node);
    }

    public override void VisitPropertyDeclaration(PropertyDeclarationSyntax node)
    {
        Symbols.Add(new Symbol
        {
            Name = node.Identifier.ValueText,
            Kind = "Property",
            Location = GetLocation(node)
        });
        base.VisitPropertyDeclaration(node);
    }

    public override void VisitFieldDeclaration(FieldDeclarationSyntax node)
    {
        foreach (var variable in node.Declaration.Variables)
        {
            Symbols.Add(new Symbol
            {
                Name = variable.Identifier.ValueText,
                Kind = "Field",
                Location = GetLocation(variable)
            });
        }
        base.VisitFieldDeclaration(node);
    }

    private static SourceLocation GetLocation(SyntaxNode node)
    {
        var lineSpan = node.GetLocation().GetLineSpan();
        return new SourceLocation
        {
            StartLine = (uint)lineSpan.StartLinePosition.Line + 1,
            StartColumn = (uint)lineSpan.StartLinePosition.Character + 1,
            EndLine = (uint)lineSpan.EndLinePosition.Line + 1,
            EndColumn = (uint)lineSpan.EndLinePosition.Character + 1,
        };
    }
}

public class Symbol
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = "";
    [JsonPropertyName("kind")]
    public string Kind { get; set; } = "";
    [JsonPropertyName("location")]
    public SourceLocation? Location { get; set; }
}

public class SourceLocation
{
    [JsonPropertyName("start_line")]
    public uint StartLine { get; set; }
    [JsonPropertyName("start_column")]
    public uint StartColumn { get; set; }
    [JsonPropertyName("end_line")]
    public uint EndLine { get; set; }
    [JsonPropertyName("end_column")]
    public uint EndColumn { get; set; }
}