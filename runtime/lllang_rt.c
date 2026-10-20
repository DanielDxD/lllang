#include "lllang_rt.h"

#include <errno.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdatomic.h>
#include <stddef.h>

/* --- Payload layouts (immediately after LlObjectHeader) --- */

typedef struct LlStringPayload {
  size_t len;
  char data[]; /* UTF-8, null-terminated at data[len] */
} LlStringPayload;

typedef struct LlArrayPayload {
  size_t count;
  size_t capacity;
  size_t elem_size;
  void (*elem_release)(void *);
  char data[];
} LlArrayPayload;

static void *ll_alloc_bytes(const LlTypeDesc *type, size_t payload_bytes);
static void ll_string_deinit(void *obj);
static void ll_array_deinit(void *obj);
void ll_trap(const char *kind, const char *msg);

const LlTypeDesc ll_string_type = {
    .name = "String",
    .deinit = ll_string_deinit,
    .size = sizeof(size_t),
};

const LlTypeDesc ll_array_type = {
    .name = "Array",
    .deinit = ll_array_deinit,
    .size = offsetof(LlArrayPayload, data),
};

/* --- Self-test (compile with -DLLLANG_RT_SELFTEST and link a stub ll_main) ---
 * gcc -DLLLANG_RT_SELFTEST -o rt_test lllang_rt.c && ./rt_test
 * Expect: hello\n42\n
 */

LlObjectHeader *ll_header(void *obj) {
  if (obj == NULL) {
    return NULL;
  }
  return (LlObjectHeader *)((char *)obj - sizeof(LlObjectHeader));
}

static void *ll_alloc_bytes(const LlTypeDesc *type, size_t payload_bytes) {
  size_t total = sizeof(LlObjectHeader) + payload_bytes;
  LlObjectHeader *hdr = (LlObjectHeader *)malloc(total);
  if (hdr == NULL) {
    ll_trap("memory", "malloc failed");
  }
  atomic_store(&hdr->refcount, (int64_t)1);
  hdr->type = type;
  memset((char *)hdr + sizeof(LlObjectHeader), 0, payload_bytes);
  return (char *)hdr + sizeof(LlObjectHeader);
}

void *ll_alloc(const LlTypeDesc *type) {
  if (type == NULL) {
    ll_trap("memory", "ll_alloc: null type");
  }
  return ll_alloc_bytes(type, type->size);
}

void *ll_alloc_sized(size_t payload_bytes) {
  static const LlTypeDesc generic = { .name = "object", .deinit = NULL, .size = 0 };
  return ll_alloc_bytes(&generic, payload_bytes);
}

void *ll_retain(void *obj) {
  if (obj == NULL) {
    return NULL;
  }
  LlObjectHeader *hdr = ll_header(obj);
  atomic_fetch_add(&hdr->refcount, (int64_t)1);
  return obj;
}

void ll_release(void *obj) {
  if (obj == NULL) {
    return;
  }
  LlObjectHeader *hdr = ll_header(obj);
  int64_t prev = atomic_fetch_sub(&hdr->refcount, (int64_t)1);
  if (prev == 1) {
    if (hdr->type != NULL && hdr->type->deinit != NULL) {
      hdr->type->deinit(obj);
    }
    free(hdr);
  }
}

static void ll_string_deinit(void *obj) {
  (void)obj;
}

static LlStringPayload *ll_string_payload(void *str) {
  return (LlStringPayload *)str;
}

void *ll_string_from_bytes(const char *s, size_t len) {
  if (s == NULL && len != 0) {
    ll_trap("string", "ll_string_from_bytes: null pointer with non-zero length");
  }
  void *obj = ll_alloc_bytes(&ll_string_type, sizeof(size_t) + len + 1);
  LlStringPayload *p = ll_string_payload(obj);
  p->len = len;
  if (len > 0) {
    memcpy(p->data, s, len);
  }
  p->data[len] = '\0';
  return obj;
}

void *ll_string_from_cstr(const char *s) {
  if (s == NULL) {
    return ll_string_from_bytes("", 0);
  }
  return ll_string_from_bytes(s, strlen(s));
}

size_t ll_string_len(void *str) {
  if (str == NULL) {
    return 0;
  }
  return ll_string_payload(str)->len;
}

const char *ll_string_cstr(void *str) {
  if (str == NULL) {
    return "";
  }
  return ll_string_payload(str)->data;
}

int ll_string_is_empty(void *str) {
  return str == NULL || ll_string_payload(str)->len == 0;
}

void *ll_string_concat(void *a, void *b) {
  size_t la = ll_string_len(a);
  size_t lb = ll_string_len(b);
  const char *sa = ll_string_cstr(a);
  const char *sb = ll_string_cstr(b);
  void *out = ll_alloc_bytes(&ll_string_type, sizeof(size_t) + la + lb + 1);
  LlStringPayload *p = ll_string_payload(out);
  p->len = la + lb;
  if (la > 0) {
    memcpy(p->data, sa, la);
  }
  if (lb > 0) {
    memcpy(p->data + la, sb, lb);
  }
  p->data[p->len] = '\0';
  return out;
}

void *ll_string_from_i64(int64_t v) {
  char buf[32];
  int n = snprintf(buf, sizeof(buf), "%lld", (long long)v);
  if (n < 0) {
    ll_trap("string", "ll_string_from_i64: format failed");
  }
  return ll_string_from_bytes(buf, (size_t)n);
}

void *ll_string_from_f64(double v) {
  char buf[64];
  int n = snprintf(buf, sizeof(buf), "%g", v);
  if (n < 0) {
    ll_trap("string", "ll_string_from_f64: format failed");
  }
  return ll_string_from_bytes(buf, (size_t)n);
}

void *ll_string_from_bool(int v) {
  return ll_string_from_cstr(v ? "true" : "false");
}

static LlArrayPayload *ll_array_payload(void *arr) {
  return (LlArrayPayload *)arr;
}

static size_t ll_array_payload_bytes(size_t capacity, size_t elem_size) {
  return offsetof(LlArrayPayload, data) + capacity * elem_size;
}

static void ll_array_deinit(void *obj) {
  LlArrayPayload *p = ll_array_payload(obj);
  if (p->elem_release == NULL) {
    return;
  }
  for (size_t i = 0; i < p->count; i++) {
    void **slot = (void **)(p->data + i * p->elem_size);
    p->elem_release(*slot);
  }
}

void *ll_array_new(size_t elem_size, void (*elem_release)(void *)) {
  if (elem_size == 0) {
    ll_trap("array", "ll_array_new: elem_size must be > 0");
  }
  const size_t initial_cap = 4;
  void *obj = ll_alloc_bytes(&ll_array_type, ll_array_payload_bytes(initial_cap, elem_size));
  LlArrayPayload *p = ll_array_payload(obj);
  p->count = 0;
  p->capacity = initial_cap;
  p->elem_size = elem_size;
  p->elem_release = elem_release;
  return obj;
}

static void ll_array_grow(LlArrayPayload *p, void **arr_obj) {
  size_t new_cap = p->capacity == 0 ? 4 : p->capacity * 2;
  void *new_obj =
      ll_alloc_bytes(&ll_array_type, ll_array_payload_bytes(new_cap, p->elem_size));
  LlArrayPayload *np = ll_array_payload(new_obj);
  np->count = p->count;
  np->capacity = new_cap;
  np->elem_size = p->elem_size;
  np->elem_release = p->elem_release;
  if (p->count > 0) {
    memcpy(np->data, p->data, p->count * p->elem_size);
  }
  ll_release(*arr_obj);
  *arr_obj = new_obj;
}

void ll_array_append(void *arr, const void *elem) {
  if (arr == NULL) {
    ll_trap("array", "ll_array_append: null array");
  }
  if (elem == NULL) {
    ll_trap("array", "ll_array_append: null element pointer");
  }
  LlArrayPayload *p = ll_array_payload(arr);
  if (p->count >= p->capacity) {
    ll_array_grow(p, &arr);
    p = ll_array_payload(arr);
  }
  void *dest = p->data + p->count * p->elem_size;
  memcpy(dest, elem, p->elem_size);
  if (p->elem_release != NULL) {
    void **slot = (void **)dest;
    ll_retain(*slot);
  }
  p->count++;
}

size_t ll_array_count(void *arr) {
  if (arr == NULL) {
    return 0;
  }
  return ll_array_payload(arr)->count;
}

void *ll_array_get_ptr(void *arr, size_t index) {
  if (arr == NULL) {
    ll_trap("array", "ll_array_get_ptr: null array");
  }
  LlArrayPayload *p = ll_array_payload(arr);
  if (index >= p->count) {
    ll_trap("array", "ll_array_get_ptr: index out of bounds");
  }
  return p->data + index * p->elem_size;
}

void ll_print(void *str) {
  const char *s = ll_string_cstr(str);
  fputs(s, stdout);
  fputc('\n', stdout);
}

void ll_trap(const char *kind, const char *msg) {
  if (kind == NULL) {
    kind = "trap";
  }
  if (msg == NULL) {
    msg = "";
  }
  fprintf(stderr, "LLLang trap [%s]: %s\n", kind, msg);
  abort();
}

int64_t ll_int_from_string(void *str, int *ok_out) {
  const char *s = ll_string_cstr(str);
  if (ok_out != NULL) {
    *ok_out = 0;
  }
  if (s == NULL || *s == '\0') {
    return 0;
  }
  errno = 0;
  char *end = NULL;
  long long v = strtoll(s, &end, 10);
  if (errno != 0 || end == s || *end != '\0') {
    return 0;
  }
  if (v < INT64_MIN || v > INT64_MAX) {
    return 0;
  }
  if (ok_out != NULL) {
    *ok_out = 1;
  }
  return (int64_t)v;
}

double ll_double_sqrt(double x) {
  return sqrt(x);
}

int main(int argc, char **argv) {
  (void)argc;
  (void)argv;
  ll_main();
  return 0;
}

#ifdef LLLANG_RT_SELFTEST
void ll_main(void) {
  void *a = ll_string_from_cstr("hello");
  void *b = ll_string_from_i64(42);
  void *c = ll_string_concat(a, ll_string_from_cstr(" "));
  ll_release(c);
  c = ll_string_concat(a, ll_string_from_cstr(" world"));
  ll_print(c);
  int ok = 0;
  int64_t n = ll_int_from_string(b, &ok);
  if (!ok) {
    ll_trap("selftest", "parse int failed");
  }
  ll_print(ll_string_from_i64(n));
  ll_release(a);
  ll_release(b);
  ll_release(c);
}
#endif
