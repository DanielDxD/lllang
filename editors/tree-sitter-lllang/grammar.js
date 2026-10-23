// Minimal Tree-sitter grammar stub for LLLang.
// TextMate (`editors/vscode/syntaxes/lllang.tmLanguage.json`) remains the
// authoritative highlighter for VS Code / Cursor; this stub is for Neovim experiments.
// Phase 1.1d: light coverage for extensions, `#defer`, and actors (full grammar Later).

module.exports = grammar({
  name: 'lllang',

  extras: $ => [/\s/, $.comment],

  rules: {
    source_file: $ => repeat($._declaration),

    _declaration: $ => choice(
      $.import_decl,
      $.namespace_decl,
      $.func_decl,
      $.struct_decl,
      $.class_decl,
      $.actor_decl,
      $.extension_decl,
      $.protocol_decl,
      $.enum_decl,
      $.comment,
    ),

    import_decl: $ => seq('import', $.path),
    namespace_decl: $ => seq('namespace', $.path),

    func_decl: $ => seq(
      optional($.access_mod),
      optional('async'),
      optional('override'),
      optional('virtual'),
      'func',
      $.identifier,
      '(',
      optional($.parameter_list),
      ')',
      optional(seq('->', $.type)),
      optional($.throws_clause),
      $.block,
    ),

    struct_decl: $ => seq(
      optional($.access_mod),
      'struct',
      $.identifier,
      optional(seq('(', $.parameter_list, ')')),
      optional($.block),
    ),

    class_decl: $ => seq(
      optional($.access_mod),
      optional('open'),
      optional('abstract'),
      optional('sealed'),
      'class',
      $.identifier,
      optional(seq('(', $.parameter_list, ')')),
      optional($.block),
    ),

    // Actors share class-like shape; keyword kept distinct for highlighting.
    actor_decl: $ => seq(
      optional($.access_mod),
      'actor',
      $.identifier,
      optional(seq('(', $.parameter_list, ')')),
      optional($.block),
    ),

    extension_decl: $ => seq(
      optional($.access_mod),
      'extension',
      $.type,
      optional(seq(':', $.type)),
      optional(seq('where', $.where_clause)),
      $.block,
    ),

    protocol_decl: $ => seq(
      optional($.access_mod),
      'protocol',
      $.identifier,
      optional($.block),
    ),

    enum_decl: $ => seq(
      optional($.access_mod),
      'enum',
      $.identifier,
      optional($.block),
    ),

    access_mod: $ => choice('public', 'private', 'internal', 'protected', 'fileprivate', 'open'),

    throws_clause: $ => seq('throws', '(', $.type, ')'),

    where_clause: $ => seq($.identifier, ':', $.type),

    parameter_list: $ => seq($.parameter, repeat(seq(',', $.parameter))),
    parameter: $ => seq($.identifier, ':', $.type),

    type: $ => choice(
      $.identifier,
      seq('[', $.type, ']'),
      seq($.type, '?'),
      seq($.identifier, '<', $.type, repeat(seq(',', $.type)), '>'),
    ),

    path: $ => seq($.identifier, repeat(seq('.', $.identifier))),

    block: $ => seq('{', repeat($._statement), '}'),

    _statement: $ => choice(
      $.defer_stmt,
      $.hash_defer,
      $.let_stmt,
      $.var_stmt,
      $.return_stmt,
      $.expr_stmt,
      $.func_decl,
      $.comment,
    ),

    // Statement-form defer and compile-time `#defer`.
    defer_stmt: $ => seq('defer', $.block),
    hash_defer: $ => seq('#defer', $.block),

    let_stmt: $ => seq('let', $.identifier, optional(seq(':', $.type)), optional(seq('=', $.expression))),
    var_stmt: $ => seq('var', $.identifier, optional(seq(':', $.type)), optional(seq('=', $.expression))),
    return_stmt: $ => seq('return', optional($.expression)),
    expr_stmt: $ => $.expression,

    expression: $ => choice(
      $.identifier,
      $.string,
      $.number,
      seq($.expression, '.', $.identifier),
      seq($.expression, '(', optional($.argument_list), ')'),
      seq('try', $.expression),
      seq('await', $.expression),
    ),

    argument_list: $ => seq($.expression, repeat(seq(',', $.expression))),

    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,
    number: $ => /\d+(\.\d+)?/,
    comment: $ => token(choice(seq('//', /.*/), seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'))),
    string: $ => /"([^"\\]|\\.)*"/,
  },
});
