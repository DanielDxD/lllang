# Operator precedence and associativity

Higher rows bind tighter (evaluated first). Operators at the same level associate as noted.

| Level | Operators / forms | Associativity | Notes |
|------:|-------------------|---------------|-------|
| 1 | `()` call, `[]` subscript, `.` member, `?.` `?[]` optional chain, postfix `!`, user postfix | Left | Primary/postfix chain |
| 2 | prefix `!` `-` `+` `~` `&` `*` | Right | Includes pointer operators in `unsafe` |
| 3 | `as` `as?` `as!` `is` | Left | Type casts and checks |
| 4 | `*` `/` `%` | Left | Multiplicative |
| 5 | `+` `-` | Left | Additive |
| 6 | `<<` `>>` | Left | Bit shifts |
| 7 | `..<` `...` | None | Range formation; non-associative |
| 8 | `<` `<=` `>` `>=` | Left | Comparison |
| 9 | `==` `!=` | Left | Equality |
| 10 | `&` | Left | Bitwise AND |
| 11 | `^` | Left | Bitwise XOR |
| 12 | `\|` | Left | Bitwise OR |
| 13 | `&&` | Left | Logical AND |
| 14 | `\|\|` | Left | Logical OR |
| 15 | `??` | Right | Nil-coalescing |
| 16 | `? :` | Right | Ternary conditional |
| 17 | `try` `await` (prefix) | Right | Applies to following expression |
| 18 | `=` `+=` `-=` `*=` `/=` `%=` `&=` `\|=` `^=` `<<=` `>>=` | Right | Assignment |
| 19 | `=>` | N/A | Not a binary operator; separates pattern from arm body |

## Infix operator groups (custom)

User-defined infix operators declare a precedence **group** in the language guide. The compiler maps groups into slots between levels 4–12. Default assignments:

| Group name | Relates to |
|------------|------------|
| `Multiplicative` | Level 4 |
| `Additive` | Level 5 |
| `Shift` | Level 6 |
| `Comparison` | Level 8 |
| `Equality` | Level 9 |
| `Conjunction` | Level 13 |
| `Disjunction` | Level 14 |

Custom prefix/postfix operators bind at levels 1–2 respectively unless annotated.

## Expression-oriented forms (not precedence rows)

These forms are parsed by statement/expression grammar, not as binary operators:

| Form | Role |
|------|------|
| `switch e { pat => expr }` | Expression switch |
| `if cond => a else => b` | Expression if |
| `do { } catch pat => expr` | Error handling with expression handler |
| `{ (x) => x + 1 }` | Closure/lambda |
| `var total: T => expr` | Computed property shorthand |

## Unary and special punctuation

| Token | Role |
|-------|------|
| `&` (prefix on expr) | Borrow / address-of in safe code; raw pointer in `unsafe` |
| `*` (prefix on expr) | Dereference in `unsafe` |
| `@` | Attribute application |
| `#` | Raw string delimiter count; conditional compilation `#if` |
| `\(...)` | String interpolation |
| `,` | Separator; does not imply precedence |
| `;` | Optional statement separator |

## Line continuation

When a line ends with one of `(`, `[`, `{`, `,`, `=>`, or an operator from levels 4–18, the expression continues on the next line. The parser does not use indentation to infer blocks.

## Related

- Full expression grammar: [grammar.ebnf](grammar.ebnf)
- Lexical rules: [../language/01-lexical-structure.md](../language/01-lexical-structure.md)
