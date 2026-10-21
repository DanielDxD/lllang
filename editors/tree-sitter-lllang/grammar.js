// Minimal Tree-sitter grammar stub for LLLang.
// TextMate (`editors/vscode/syntaxes/lllang.tmLanguage.json`) remains the
// authoritative highlighter for VS Code / Cursor; this stub is for Neovim experiments.

module.exports = grammar({
  name: 'lllang',

  extras: $ => [/\s/, $.comment],

  rules: {
    source_file: $ => repeat($._statement),

    _statement: $ => choice(
      $.import_decl,
      $.namespace_decl,
      $.func_decl,
      $.struct_decl,
      $.comment,
    ),

    import_decl: $ => seq('import', $.path),
    namespace_decl: $ => seq('namespace', $.path),
    func_decl: $ => seq(
      optional('public'),
      optional('async'),
      'func',
      $.identifier,
      '(',
      optional($.parameter_list),
      ')',
      optional(seq('->', $.type)),
      $.block,
    ),
    struct_decl: $ => seq(
      optional('public'),
      'struct',
      $.identifier,
      optional(seq('(', $.parameter_list, ')')),
      optional($.block),
    ),

    parameter_list: $ => seq($.parameter, repeat(seq(',', $.parameter))),
    parameter: $ => seq($.identifier, ':', $.type),

    type: $ => choice($.identifier, seq('[', $.type, ']'), seq($.type, '?')),
    path: $ => seq($.identifier, repeat(seq('.', $.identifier))),
    block: $ => seq('{', repeat($._statement), '}'),

    identifier: $ => /[A-Za-z_][A-Za-z0-9_]*/,
    comment: $ => token(choice(seq('//', /.*/), seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'))),
    string: $ => /"([^"\\]|\\.)*"/,
  },
});
