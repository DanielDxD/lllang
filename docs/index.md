# LLLang documentation

## Getting started

- [Installation](getting-started/installation.md)
- [Hello, world](getting-started/hello-world.md)
- [Project anatomy](getting-started/project-anatomy.md)

## Language guide

1. [Lexical structure](language/01-lexical-structure.md)
2. [Types](language/02-types.md)
3. [Declarations](language/03-declarations.md)
4. [Expressions](language/04-expressions.md)
5. [Statements](language/05-statements.md)
6. [Structs, classes, enums](language/06-structs-classes-enums.md)
7. [Protocols & extensions](language/07-protocols-extensions.md)
8. [Generics](language/08-generics.md)
9. [Optionals](language/09-optionals.md)
10. [Error handling](language/10-error-handling.md)
11. [Memory model](language/11-memory-model.md)
12. [Concurrency](language/12-concurrency.md)
13. [Pattern matching](language/13-pattern-matching.md)
14. [Attributes](language/14-attributes.md)
15. [Namespaces & modules](language/15-namespaces-modules.md)
16. [Interop, FFI & unsafe](language/16-interop-ffi-unsafe.md)
17. [Codable](language/17-codable.md)

## Formal specification

- [Grammar (EBNF)](spec/grammar.ebnf)
- [Keywords](spec/keywords.md)
- [Operators](spec/operators.md)
- [Diagnostics](spec/diagnostics.md)

## Formats

- [`.ll` source](formats/ll.md)
- [`.bll` binary library](formats/bll.md)
- [`.sbl` type declarations](formats/sbl.md)
- [`.mll` LLM metadata](formats/mll.md)

## Toolchain

- [CLI reference](toolchain/cli-reference.md)
- [Manifest (`lang.toml`)](toolchain/manifest.md)
- [Templates](toolchain/templates.md)
- [Build pipeline](toolchain/build-pipeline.md)
- [Testing](toolchain/testing.md)
- [Formatter](toolchain/formatter.md)
- [Language Server](toolchain/lsp.md)
- [Editor support](toolchain/editor-support.md)

## Packages

- [Package manager](packages/package-manager.md)
- [Registry protocol](packages/registry-protocol.md)
- [Versioning & ABI](packages/versioning-and-abi.md)

## Standard library

- [Overview](stdlib/overview.md)

## LLM

- [Overview](llm/overview.md)
- [Context generation](llm/context-generation.md)
- [`Std.Llm`](llm/std-llm.md)
- [Tools & structured output](llm/tools-and-structured-output.md)
- [MCP](llm/mcp.md)

## Design

- [Philosophy](design/philosophy.md)
- [Swift vs C# vs LLLang](design/swift-vs-csharp-vs-lllang.md)
- [Toolchain architecture](design/toolchain-architecture.md)
- [ADRs](design/adr/)
