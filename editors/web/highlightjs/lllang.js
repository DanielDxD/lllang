/**
 * highlight.js language definition for LLLang.
 * Usage: hljs.registerLanguage('lllang', lllangGrammar); hljs.highlightAll();
 */
(function (factory) {
  if (typeof module !== "undefined" && module.exports) {
    module.exports = factory;
  } else if (typeof window !== "undefined" && window.hljs) {
    window.hljs.registerLanguage("lllang", factory(window.hljs));
  }
})(function (hljs) {
  const KEYWORDS = [
    "namespace",
    "import",
    "public",
    "private",
    "protected",
    "internal",
    "fileprivate",
    "struct",
    "class",
    "enum",
    "protocol",
    "extension",
    "func",
    "let",
    "var",
    "const",
    "if",
    "else",
    "elseif",
    "guard",
    "while",
    "repeat",
    "for",
    "in",
    "return",
    "throw",
    "throws",
    "try",
    "catch",
    "do",
    "defer",
    "switch",
    "default",
    "where",
    "is",
    "as",
    "async",
    "await",
    "weak",
    "unowned",
    "override",
    "virtual",
    "abstract",
    "sealed",
    "init",
    "deinit",
    "self",
    "Self",
    "super",
    "inout",
    "get",
    "set",
    "didSet",
    "willSet",
    "unsafe",
    "static",
    "mut",
    "break",
    "continue",
    "fallthrough",
    "and",
    "or",
    "not",
    "true",
    "false",
    "nil",
  ].join(" ");

  const TYPES =
    "Void Never Any Bool String Character Int Int8 Int16 Int32 Int64 UInt Float Double Optional Result Array Dictionary Set Codable Equatable Hashable Sendable";

  return {
    name: "LLLang",
    aliases: ["ll", "lllang"],
    keywords: {
      keyword: KEYWORDS,
      type: TYPES,
      literal: "true false nil",
    },
    contains: [
      hljs.C_LINE_COMMENT_MODE,
      hljs.C_BLOCK_COMMENT_MODE,
      {
        scope: "meta",
        begin: /@[A-Za-z_]\w*/,
      },
      {
        scope: "string",
        begin: /"""/,
        end: /"""/,
        contains: [
          {
            scope: "subst",
            begin: /\\\(/,
            end: /\)/,
            contains: [hljs.BACKSLASH_ESCAPE],
          },
          hljs.BACKSLASH_ESCAPE,
        ],
      },
      {
        scope: "string",
        begin: /"/,
        end: /"/,
        illegal: /\n/,
        contains: [
          {
            scope: "subst",
            begin: /\\\(/,
            end: /\)/,
            contains: [hljs.BACKSLASH_ESCAPE],
          },
          hljs.BACKSLASH_ESCAPE,
        ],
      },
      {
        scope: "number",
        begin: /\b0[xX][0-9A-Fa-f_]+\b|\b0[bB][01_]+\b|\b[0-9][0-9_]*(\.[0-9_]+)?([eE][+-]?[0-9_]+)?\b/,
        relevance: 0,
      },
      {
        scope: "title.class",
        begin: /\b[A-Z][A-Za-z0-9_]*\b/,
      },
      {
        scope: "operator",
        begin: /(-?>|=>|->|\?\?|\?\.|\.\.\.|\.\.<|===|!==|==|!=|<=|>=|&&|\|\||<<|>>|[+\-*/%=<>!&|^~?:])/,
      },
    ],
  };
});
