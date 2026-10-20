# Reserved keywords

LLLang reserves the following identifiers. They cannot be used as unescaped names for types, variables, or functions. Escape with backticks (`` `protocol` ``) only for interop shims.

Keywords are grouped by role. Modifiers combine according to the grammar (see [grammar.ebnf](grammar.ebnf)).

## Declarations & types

| Keyword | Meaning |
|---------|---------|
| `namespace` | Declares a dotted module path for the rest of the file or block |
| `import` | Brings another module or symbol into scope |
| `struct` | Value type with copy semantics |
| `class` | Reference type managed by ARC |
| `enum` | Algebraic sum type; cases omit the `case` keyword |
| `protocol` | Interface / trait with requirements and optional defaults |
| `extension` | Adds members or conformances to an existing type |
| `typealias` | Synonym for an existing type |
| `func` | Function or method |
| `init` | Constructor |
| `deinit` | Deterministic teardown for class instances |
| `subscript` | Indexed access operator |
| `operator` | User-defined prefix/postfix/infix operator |
| `var` | Mutable binding or mutable member |
| `let` | Immutable binding or immutable member |
| `const` | Compile-time constant binding |
| `static` | Type-scoped member (when used on members) |
| `type` | Modifier for type-level members (`type func`, etc., when present in guide) |

## Access control

| Keyword | Meaning |
|---------|---------|
| `public` | Visible outside defining module |
| `internal` | Visible within package (default for unmarked declarations) |
| `private` | Visible only within enclosing declaration |
| `fileprivate` | Visible only within source file |

## Parameter & capture modifiers

| Keyword | Meaning |
|---------|---------|
| `inout` | Pass-by-reference for mutable parameters |
| `weak` | Non-owning reference to a class (optional when target may deallocate) |
| `unowned` | Non-owning reference assumed valid for lifetime |
| `self` | Current instance |
| `Self` | Dynamic type of current context (type metatype in protocols) |
| `super` | Superclass method dispatch |

## Control flow

| Keyword | Meaning |
|---------|---------|
| `if` | Conditional; also expression form with `=>` arms |
| `else` | Alternative branch |
| `elseif` | Else-if branch |
| `switch` | Expression or statement pattern match with `=>` arms |
| `default` | Fallback arm in `switch` |
| `where` | Constraint on patterns, generics, or loops |
| `guard` | Early exit unless condition holds |
| `while` | Pre-test loop |
| `repeat` | Post-test loop |
| `for` | Iterator loop |
| `in` | Iterator membership (`for x in xs`) |
| `break` | Exit loop or switch |
| `continue` | Next iteration |
| `return` | Exit function with optional value |
| `throw` | Raise a typed error |
| `throws` | Function may propagate listed error types |
| `try` | Invoke throwing call; required at call site per rules |
| `catch` | Handle errors in `do` or postfix `catch` expression |
| `do` | Scope for error handling |
| `defer` | Run block on scope exit |
| `fallthrough` | Continue to next switch arm (statement switch only) |

## Optionals & logic

| Keyword | Meaning |
|---------|---------|
| `nil` | Absence of value for an optional type |
| `true` | Boolean true |
| `false` | Boolean false |
| `as` | Cast to type |
| `as?` | Failing cast producing optional |
| `as!` | Force cast (trap on failure) |
| `is` | Type check predicate |

## Concurrency (planned runtime)

| Keyword | Meaning |
|---------|---------|
| `async` | Asynchronous function |
| `await` | Suspend until async operation completes |
| `actor` | Isolated reference type (later phase) |
| `isolated` | Isolation annotation for actors |
| `nonisolated` | Opt out of actor isolation |

## Memory & safety

| Keyword | Meaning |
|---------|---------|
| `unsafe` | Block or call that opts out of memory safety checks |

## Generics

| Keyword | Meaning |
|---------|---------|
| `where` | Generic constraints (also used in patterns) |

## Conditional compilation

| Keyword | Meaning |
|---------|---------|
| `#if` | Compile-time branch |
| `#elseif` | Alternate compile-time branch |
| `#else` | Default compile-time branch |
| `#endif` | End conditional block |

## Literals & special forms

| Keyword | Meaning |
|---------|---------|
| `_` | Wildcard pattern or ignored parameter name |

## Explicitly **not** keywords

LLLang deliberately omits several familiar tokens:

| Absent | Replacement / rationale |
|--------|-------------------------|
| `null` | Use `nil` inside optionals only |
| `new` | Call `Type(args)` |
| `case` (enum) | Bare case names: `success, failure(Error)` |
| `mutating` | Struct methods mutate by default when not marked `let` on `self` binding rules |
| `associatedtype` | Generic protocol parameters: `protocol P<T>` |
| `any` / `some` | Use concrete generics or protocol existentials as designed in guide |
| Swift external labels at defs | Single name per parameter; call sites use argument labels only when mirrored in API design docs |

## Contextual keywords

These are keywords only in specific grammatical positions:

| Token | Context |
|-------|---------|
| `=>` | Switch/if arm, lambda body, computed property shorthand |
| `->` | Function return type |
| `?` | Optional type suffix or optional chaining |
| `!` | Force unwrap postfix |
| `@` | Start of attribute |

See [operators](operators.md) for symbolic tokens and precedence.
