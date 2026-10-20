#ifndef LLLANG_RT_H
#define LLLANG_RT_H

#include <stddef.h>
#include <stdint.h>
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
void *ll_retain(void *obj);
void ll_release(void *obj);

LlObjectHeader *ll_header(void *obj);

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

/* User program entry (defined by compiled LLLang). */
void ll_main(void);

#endif /* LLLANG_RT_H */
