#ifndef LLLANG_RT_H
#define LLLANG_RT_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdatomic.h>

typedef struct LlTypeDesc {
  const char *name;
  void (*deinit)(void *obj); /* payload pointer; may be NULL */
  size_t size;               /* fixed payload prefix size (flex arrays follow) */
} LlTypeDesc;

typedef struct LlObjectHeader {
  _Atomic int64_t refcount;
  const LlTypeDesc *type;
} LlObjectHeader;

/* --- ARC (payload pointer API) --- */

void *ll_alloc(const LlTypeDesc *type);
void *ll_alloc_sized(size_t payload_bytes);
void *ll_retain(void *obj);
void ll_release(void *obj);
int ll_is_unique(void *obj);
void *ll_make_unique(void *obj, size_t payload_bytes);

/* Box a value-type error payload for the throwing ABI (`__err_out`). */
void *ll_box_bytes(const void *data, size_t len);
void ll_unbox_bytes(void *box, void *out, size_t len);

LlObjectHeader *ll_header(void *obj);

/* Class vtable: first word of class payload may hold a vtable pointer. */
typedef void (*LlVTableFn)(void);

/* --- Strings --- */

extern const LlTypeDesc ll_string_type;

void *ll_string_from_cstr(const char *s);
void *ll_string_from_bytes(const char *s, size_t len);
size_t ll_string_len(void *str);
const char *ll_string_cstr(void *str);
int ll_string_is_empty(void *str);
void *ll_string_concat(void *a, void *b);
void *ll_string_from_i64(int64_t v);
void *ll_string_from_f64(double v);
void *ll_string_from_bool(int v);
int ll_string_eq(void *a, void *b);

/* --- Arrays --- */

extern const LlTypeDesc ll_array_type;

void *ll_array_new(size_t elem_size, void (*elem_release)(void *));
void ll_array_append(void *arr, const void *elem);
size_t ll_array_count(void *arr);
void *ll_array_get_ptr(void *arr, size_t index);

/* --- IO / traps / Int --- */

void ll_print(void *str);
void ll_trap(const char *kind, const char *msg);
int64_t ll_int_from_string(void *str, int *ok_out);
double ll_double_sqrt(double x);

/* --- JSON / TOON MVP (schema: "field:s,field:i,...") --- */
int ll_json_decode_fields(void *json_str, const char *schema, void *out_struct);
void *ll_json_encode_fields(const char *schema, const void *in_struct);
int ll_toon_decode_fields(void *toon_str, const char *schema, void *out_struct);
void *ll_toon_encode_fields(const char *schema, const void *in_struct);

/* --- HTTP / LLM MVP --- */
void *ll_http_get(void *url_str, int *ok_out);
void *ll_llm_run_agent(void *model_str, void *tools_str, void *user_str);
void *ll_llm_merge_context(void *package_name);
void ll_noop(void);

/* --- Testing --- */
void ll_test_expect(bool ok, void *msg);
void ll_test_expect_empty(void *str, void *msg);
void ll_test_expect_not_empty(void *str, void *msg);
void ll_test_contains(void *hay, void *needle, void *msg);
void ll_test_approx(double a, double b, void *msg);
int64_t ll_test_job_id(void);

/* User program entry (defined by compiled LLLang). */
void ll_main(void);

#endif /* LLLANG_RT_H */
