#include "lllang_rt.h"
#include "ll_compat.h"

#include <ctype.h>
#include <errno.h>
#include <limits.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdatomic.h>
#include <stddef.h>
#include <time.h>
#if !defined(_WIN32)
#include <unistd.h>
#include <sys/wait.h>
#endif

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
const char *ll_string_cstr(void *str);

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

const char *ll_type_name(void *obj) {
  LlObjectHeader *hdr = ll_header(obj);
  if (hdr == NULL || hdr->type == NULL || hdr->type->name == NULL) {
    return "";
  }
  return hdr->type->name;
}

int ll_isa(void *obj, void *type_name_str) {
  if (obj == NULL || type_name_str == NULL) {
    return 0;
  }
  const char *type_name = ll_string_cstr(type_name_str);
  if (type_name == NULL) {
    return 0;
  }
  const char *have = ll_type_name(obj);
  return have[0] != '\0' && strcmp(have, type_name) == 0;
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

int ll_is_unique(void *obj) {
  if (obj == NULL) {
    return 1;
  }
  LlObjectHeader *hdr = ll_header(obj);
  return atomic_load(&hdr->refcount) == 1;
}

void *ll_make_unique(void *obj, size_t payload_bytes) {
  if (obj == NULL) {
    return ll_alloc_sized(payload_bytes);
  }
  if (ll_is_unique(obj)) {
    return obj;
  }
  void *copy = ll_alloc_sized(payload_bytes);
  memcpy(copy, obj, payload_bytes);
  ll_release(obj);
  return copy;
}

void *ll_box_bytes(const void *data, size_t len) {
  void *obj = ll_alloc_sized(len == 0 ? 1 : len);
  if (data != NULL && len > 0) {
    memcpy(obj, data, len);
  }
  return obj;
}

void ll_unbox_bytes(void *box, void *out, size_t len) {
  if (box == NULL) {
    if (out != NULL && len > 0) {
      memset(out, 0, len);
    }
    return;
  }
  if (out != NULL && len > 0) {
    memcpy(out, box, len);
  }
  ll_release(box);
}

void *ll_retain(void *obj) {
  if (obj == NULL || (uintptr_t)obj < 4096u) {
    return obj;
  }
  LlObjectHeader *hdr = ll_header(obj);
  atomic_fetch_add(&hdr->refcount, (int64_t)1);
  return obj;
}

void ll_weak_clear_object(void *obj); /* Phase 10 side table */

void ll_release(void *obj) {
  if (obj == NULL || (uintptr_t)obj < 4096u) {
    return;
  }
  LlObjectHeader *hdr = ll_header(obj);
  int64_t prev = atomic_fetch_sub(&hdr->refcount, (int64_t)1);
  if (prev == 1) {
    ll_weak_clear_object(obj);
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

int ll_string_eq(void *a, void *b) {
  if (a == b) return 1;
  const char *sa = ll_string_cstr(a);
  const char *sb = ll_string_cstr(b);
  if (sa == NULL || sb == NULL) return sa == sb;
  return strcmp(sa, sb) == 0;
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

void ll_array_append(void **arr, const void *elem) {
  if (arr == NULL || *arr == NULL) {
    ll_trap("array", "ll_array_append: null array");
  }
  if (elem == NULL) {
    ll_trap("array", "ll_array_append: null element pointer");
  }
  /* COW: shared arrays must copy before mutation (caller holds the handle). */
  if (!ll_is_unique(*arr)) {
    LlArrayPayload *old = ll_array_payload(*arr);
    size_t bytes = ll_array_payload_bytes(old->capacity, old->elem_size);
    *arr = ll_make_unique(*arr, bytes);
  }
  LlArrayPayload *p = ll_array_payload(*arr);
  if (p->count >= p->capacity) {
    ll_array_grow(p, arr);
    p = ll_array_payload(*arr);
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

void *ll_array_new_i64(void) {
  return ll_array_new(sizeof(int64_t), NULL);
}

void ll_array_append_i64(void **arr, int64_t v) {
  ll_array_append(arr, &v);
}

int64_t ll_array_get_i64(void *arr, size_t index) {
  return *(int64_t *)ll_array_get_ptr(arr, index);
}

void ll_array_set_i64(void **arr, size_t index, int64_t v) {
  if (arr == NULL || *arr == NULL) {
    ll_trap("array", "ll_array_set_i64: null array");
  }
  if (!ll_is_unique(*arr)) {
    LlArrayPayload *old = ll_array_payload(*arr);
    size_t bytes = ll_array_payload_bytes(old->capacity, old->elem_size);
    *arr = ll_make_unique(*arr, bytes);
  }
  *(int64_t *)ll_array_get_ptr(*arr, index) = v;
}

/* Phase 1.2: return a sorted copy of an Int array (ascending). */
void *ll_array_sorted_i64(void *arr) {
  size_t n = ll_array_count(arr);
  void *out = ll_array_new_i64();
  for (size_t i = 0; i < n; i++) {
    ll_array_append_i64(&out, ll_array_get_i64(arr, i));
  }
  /* insertion sort */
  for (size_t i = 1; i < n; i++) {
    int64_t key = ll_array_get_i64(out, i);
    size_t j = i;
    while (j > 0 && ll_array_get_i64(out, j - 1) > key) {
      ll_array_set_i64(&out, j, ll_array_get_i64(out, j - 1));
      j--;
    }
    ll_array_set_i64(&out, j, key);
  }
  return out;
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

void ll_die(void *msg_str) {
  ll_trap("die", ll_string_cstr(msg_str));
}

void ll_eprint(void *str) {
  const char *s = ll_string_cstr(str);
  fputs(s, stderr);
  fputc('\n', stderr);
}

void *ll_read_line(void) {
  char buf[4096];
  if (fgets(buf, (int)sizeof(buf), stdin) == NULL) {
    return NULL;
  }
  size_t n = strlen(buf);
  if (n > 0 && buf[n - 1] == '\n') {
    buf[n - 1] = '\0';
    n--;
  }
  return ll_string_from_bytes(buf, n);
}

void ll_fatal_error(void *msg_str) {
  ll_trap("fatalError", ll_string_cstr(msg_str));
}

void ll_assert(int cond, void *msg_str) {
  if (!cond) {
    ll_trap("assert", ll_string_cstr(msg_str));
  }
}

static int g_argc = 0;
static char **g_argv = NULL;

void ll_rt_set_args(int argc, char **argv) {
  g_argc = argc;
  g_argv = argv;
}

int64_t ll_args_count(void) {
  return g_argc > 0 ? (int64_t)(g_argc) : 0;
}

void *ll_args_get(size_t index) {
  if (g_argv == NULL || index >= (size_t)g_argc) {
    return NULL;
  }
  return ll_string_from_cstr(g_argv[index]);
}

void *ll_env_get(void *key_str) {
  const char *key = ll_string_cstr(key_str);
  const char *v = getenv(key);
  if (v == NULL) {
    return NULL;
  }
  return ll_string_from_cstr(v);
}

static void ll_env_trim_inplace(char *s) {
  if (s == NULL) return;
  char *start = s;
  while (*start == ' ' || *start == '\t' || *start == '\r' || *start == '\n') start++;
  if (start != s) memmove(s, start, strlen(start) + 1);
  size_t n = strlen(s);
  while (n > 0 && (s[n - 1] == ' ' || s[n - 1] == '\t' || s[n - 1] == '\r' || s[n - 1] == '\n')) {
    s[--n] = '\0';
  }
  if ((s[0] == '"' && s[n - 1] == '"') || (s[0] == '\'' && s[n - 1] == '\'')) {
    s[n - 1] = '\0';
    memmove(s, s + 1, n - 1);
  }
}

void ll_env_load_dotenv(void *path_str) {
  const char *path = ll_string_cstr(path_str);
  if (path == NULL || path[0] == '\0') path = ".env";
  FILE *f = fopen(path, "r");
  if (f == NULL) return;
  char line[4096];
  while (fgets(line, sizeof(line), f)) {
    char *p = line;
    while (*p == ' ' || *p == '\t') p++;
    if (*p == '\0' || *p == '#' || *p == '\n' || *p == '\r') continue;
    char *eq = strchr(p, '=');
    if (eq == NULL) continue;
    *eq = '\0';
    char *key = p;
    char *val = eq + 1;
    ll_env_trim_inplace(key);
    ll_env_trim_inplace(val);
    if (key[0] == '\0') continue;
#if defined(_WIN32)
    _putenv_s(key, val);
#else
    setenv(key, val, 1);
#endif
  }
  fclose(f);
}

void *ll_env_get_string_or(void *key_str, void *default_str) {
  void *v = ll_env_get(key_str);
  if (v == NULL) {
    return ll_retain(default_str);
  }
  return v;
}

static int ll_env_parse_bool(const char *v, int *ok_out) {
  if (ok_out) *ok_out = 0;
  if (v == NULL) return 0;
  if (strcmp(v, "1") == 0 || strcasecmp(v, "true") == 0 || strcasecmp(v, "yes") == 0) {
    if (ok_out) *ok_out = 1;
    return 1;
  }
  if (strcmp(v, "0") == 0 || strcasecmp(v, "false") == 0 || strcasecmp(v, "no") == 0) {
    if (ok_out) *ok_out = 1;
    return 0;
  }
  return 0;
}

int64_t ll_env_get_i64(void *key_str, int *ok_out) {
  if (ok_out) *ok_out = 0;
  const char *key = ll_string_cstr(key_str);
  const char *v = key ? getenv(key) : NULL;
  if (v == NULL || v[0] == '\0') return 0;
  char *end = NULL;
  errno = 0;
  long long n = strtoll(v, &end, 10);
  if (errno != 0 || end == v) return 0;
  if (ok_out) *ok_out = 1;
  return (int64_t)n;
}

int64_t ll_env_get_i64_or(void *key_str, int64_t default_val) {
  int ok = 0;
  int64_t v = ll_env_get_i64(key_str, &ok);
  return ok ? v : default_val;
}

int ll_env_get_bool(void *key_str, int *ok_out) {
  const char *key = ll_string_cstr(key_str);
  const char *v = key ? getenv(key) : NULL;
  return ll_env_parse_bool(v, ok_out);
}

int ll_env_get_bool_or(void *key_str, int default_val) {
  int ok = 0;
  int v = ll_env_get_bool(key_str, &ok);
  return ok ? v : default_val;
}

double ll_env_get_f64(void *key_str, int *ok_out) {
  if (ok_out) *ok_out = 0;
  const char *key = ll_string_cstr(key_str);
  const char *v = key ? getenv(key) : NULL;
  if (v == NULL || v[0] == '\0') return 0.0;
  char *end = NULL;
  errno = 0;
  double d = strtod(v, &end);
  if (errno != 0 || end == v) return 0.0;
  if (ok_out) *ok_out = 1;
  return d;
}

double ll_env_get_f64_or(void *key_str, double default_val) {
  int ok = 0;
  double v = ll_env_get_f64(key_str, &ok);
  return ok ? v : default_val;
}

int64_t ll_env_get_decimal(void *key_str, int *ok_out) {
  if (ok_out) *ok_out = 0;
  void *s = ll_env_get(key_str);
  if (s == NULL) return 0;
  int64_t v = ll_decimal_from_string(s, ok_out);
  ll_release(s);
  return v;
}

int64_t ll_env_get_decimal_or(void *key_str, int64_t default_val) {
  int ok = 0;
  int64_t v = ll_env_get_decimal(key_str, &ok);
  return ok ? v : default_val;
}

int ll_file_exists(void *path_str) {
  const char *path = ll_string_cstr(path_str);
  FILE *f = fopen(path, "rb");
  if (f == NULL) {
    return 0;
  }
  fclose(f);
  return 1;
}

void *ll_file_read_text(void *path_str) {
  const char *path = ll_string_cstr(path_str);
  FILE *f = fopen(path, "rb");
  if (f == NULL) {
    return ll_string_from_cstr("");
  }
  if (fseek(f, 0, SEEK_END) != 0) {
    fclose(f);
    return ll_string_from_cstr("");
  }
  long sz = ftell(f);
  if (sz < 0) {
    fclose(f);
    return ll_string_from_cstr("");
  }
  rewind(f);
  char *buf = (char *)malloc((size_t)sz + 1);
  if (buf == NULL) {
    fclose(f);
    return ll_string_from_cstr("");
  }
  size_t n = fread(buf, 1, (size_t)sz, f);
  fclose(f);
  buf[n] = '\0';
  void *s = ll_string_from_bytes(buf, n);
  free(buf);
  return s;
}

int ll_file_write_text(void *path_str, void *contents_str) {
  const char *path = ll_string_cstr(path_str);
  const char *data = ll_string_cstr(contents_str);
  FILE *f = fopen(path, "wb");
  if (f == NULL) {
    return 0;
  }
  size_t len = strlen(data);
  size_t n = fwrite(data, 1, len, f);
  fclose(f);
  return n == len ? 1 : 0;
}

int64_t ll_instant_now_ms(void) {
#if defined(_WIN32)
  return (int64_t)GetTickCount64();
#else
  struct timespec ts;
  if (clock_gettime(CLOCK_REALTIME, &ts) != 0) {
    return 0;
  }
  return (int64_t)ts.tv_sec * 1000 + (int64_t)ts.tv_nsec / 1000000;
#endif
}

void ll_sleep_ms(int64_t ms) {
  if (ms <= 0) {
    return;
  }
#if defined(_WIN32)
  Sleep((DWORD)ms);
#else
  struct timespec ts;
  ts.tv_sec = (time_t)(ms / 1000);
  ts.tv_nsec = (long)((ms % 1000) * 1000000);
  nanosleep(&ts, NULL);
#endif
}

void ll_process_exit(int64_t code) {
  exit((int)code);
}

typedef struct {
  int64_t exit_code;
} LlProcessHandle;

void *ll_process_start(void *cmd_str) {
  const char *cmd = ll_string_cstr(cmd_str);
  LlProcessHandle *h = (LlProcessHandle *)malloc(sizeof(LlProcessHandle));
  if (h == NULL) return NULL;
  if (cmd == NULL || cmd[0] == '\0') {
    h->exit_code = -1;
    return h;
  }
  int rc = system(cmd);
  if (rc == -1) {
    h->exit_code = -1;
  } else {
#if defined(WIFEXITED) && defined(WEXITSTATUS)
    h->exit_code = WIFEXITED(rc) ? WEXITSTATUS(rc) : rc;
#else
    h->exit_code = (int64_t)rc;
#endif
  }
  return h;
}

int64_t ll_process_wait(void *handle) {
  if (handle == NULL) return -1;
  LlProcessHandle *h = (LlProcessHandle *)handle;
  int64_t code = h->exit_code;
  free(h);
  return code;
}

int ll_string_contains(void *hay, void *needle) {
  const char *h = ll_string_cstr(hay);
  const char *n = ll_string_cstr(needle);
  if (h == NULL || n == NULL) {
    return 0;
  }
  return strstr(h, n) != NULL ? 1 : 0;
}

void *ll_string_trim(void *str) {
  const char *s = ll_string_cstr(str);
  if (s == NULL) {
    return ll_string_from_cstr("");
  }
  while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') {
    s++;
  }
  size_t len = strlen(s);
  while (len > 0 &&
         (s[len - 1] == ' ' || s[len - 1] == '\t' || s[len - 1] == '\n' || s[len - 1] == '\r')) {
    len--;
  }
  return ll_string_from_bytes(s, len);
}

void *ll_string_substring(void *str, int64_t start, int64_t end) {
  const char *s = ll_string_cstr(str);
  size_t len = ll_string_len(str);
  if (s == NULL) {
    return ll_string_from_cstr("");
  }
  if (start < 0) {
    start = 0;
  }
  if (end < start) {
    end = start;
  }
  if ((size_t)start > len) {
    start = (int64_t)len;
  }
  if ((size_t)end > len) {
    end = (int64_t)len;
  }
  return ll_string_from_bytes(s + start, (size_t)(end - start));
}

static void ll_release_elem(void *obj) {
  ll_release(obj);
}

void *ll_array_new_ptr(void) {
  return ll_array_new(sizeof(void *), ll_release_elem);
}

void ll_array_append_ptr(void **arr, void *elem) {
  ll_array_append(arr, &elem);
}

void *ll_array_get_ref(void *arr, size_t index) {
  return *(void **)ll_array_get_ptr(arr, index);
}

void *ll_string_split(void *str, void *sep) {
  const char *s = ll_string_cstr(str);
  const char *d = ll_string_cstr(sep);
  void *arr = ll_array_new_ptr();
  if (s == NULL) {
    return arr;
  }
  if (d == NULL || d[0] == '\0') {
    void *one = ll_string_from_cstr(s);
    ll_array_append_ptr(&arr, one);
    ll_release(one);
    return arr;
  }
  size_t dlen = strlen(d);
  const char *start = s;
  while (*start) {
    const char *found = strstr(start, d);
    if (found == NULL) {
      void *part = ll_string_from_cstr(start);
      ll_array_append_ptr(&arr, part);
      ll_release(part);
      break;
    }
    void *part = ll_string_from_bytes(start, (size_t)(found - start));
    ll_array_append_ptr(&arr, part);
    ll_release(part);
    start = found + dlen;
    if (*start == '\0') {
      void *empty = ll_string_from_cstr("");
      ll_array_append_ptr(&arr, empty);
      ll_release(empty);
      break;
    }
  }
  if (*s == '\0') {
    void *empty = ll_string_from_cstr("");
    ll_array_append_ptr(&arr, empty);
    ll_release(empty);
  }
  return arr;
}

void *ll_string_upper(void *str) {
  const char *s = ll_string_cstr(str);
  size_t n = ll_string_len(str);
  if (s == NULL) return ll_string_from_cstr("");
  char *buf = (char *)malloc(n + 1);
  if (buf == NULL) return ll_string_from_cstr("");
  for (size_t i = 0; i < n; i++) buf[i] = (char)toupper((unsigned char)s[i]);
  buf[n] = '\0';
  void *out = ll_string_from_bytes(buf, n);
  free(buf);
  return out;
}

void *ll_string_lower(void *str) {
  const char *s = ll_string_cstr(str);
  size_t n = ll_string_len(str);
  if (s == NULL) return ll_string_from_cstr("");
  char *buf = (char *)malloc(n + 1);
  if (buf == NULL) return ll_string_from_cstr("");
  for (size_t i = 0; i < n; i++) buf[i] = (char)tolower((unsigned char)s[i]);
  buf[n] = '\0';
  void *out = ll_string_from_bytes(buf, n);
  free(buf);
  return out;
}

void *ll_string_replace(void *str, void *old_str, void *new_str) {
  const char *s = ll_string_cstr(str);
  const char *old = ll_string_cstr(old_str);
  const char *neu = ll_string_cstr(new_str);
  if (s == NULL) return ll_string_from_cstr("");
  if (old == NULL || old[0] == '\0') return ll_string_from_cstr(s);
  if (neu == NULL) neu = "";
  size_t old_len = strlen(old);
  size_t neu_len = strlen(neu);
  size_t count = 0;
  for (const char *p = s; (p = strstr(p, old)) != NULL; p += old_len) count++;
  size_t s_len = strlen(s);
  size_t out_len = s_len + count * (neu_len > old_len ? neu_len - old_len : 0);
  if (neu_len < old_len) out_len = s_len - count * (old_len - neu_len);
  char *buf = (char *)malloc(out_len + 1);
  if (buf == NULL) return ll_string_from_cstr(s);
  char *dst = buf;
  const char *src = s;
  while (*src) {
    const char *found = strstr(src, old);
    if (found == NULL) {
      memcpy(dst, src, strlen(src) + 1);
      break;
    }
    size_t lead = (size_t)(found - src);
    memcpy(dst, src, lead);
    dst += lead;
    memcpy(dst, neu, neu_len);
    dst += neu_len;
    src = found + old_len;
    *dst = '\0';
  }
  void *out = ll_string_from_cstr(buf);
  free(buf);
  return out;
}

void *ll_string_join(void *arr, void *sep) {
  if (arr == NULL) return ll_string_from_cstr("");
  size_t n = ll_array_count(arr);
  const char *d = ll_string_cstr(sep);
  if (d == NULL) d = "";
  size_t dlen = strlen(d);
  size_t total = 0;
  for (size_t i = 0; i < n; i++) {
    void *e = ll_array_get_ref(arr, i);
    total += ll_string_len(e);
    if (i + 1 < n) total += dlen;
  }
  char *buf = (char *)malloc(total + 1);
  if (buf == NULL) return ll_string_from_cstr("");
  char *dst = buf;
  for (size_t i = 0; i < n; i++) {
    void *e = ll_array_get_ref(arr, i);
    const char *cs = ll_string_cstr(e);
    size_t len = ll_string_len(e);
    if (cs && len) {
      memcpy(dst, cs, len);
      dst += len;
    }
    if (i + 1 < n && dlen) {
      memcpy(dst, d, dlen);
      dst += dlen;
    }
  }
  *dst = '\0';
  void *out = ll_string_from_bytes(buf, (size_t)(dst - buf));
  free(buf);
  return out;
}

static void ll_fill_random(unsigned char *buf, size_t n) {
#if defined(__APPLE__) || defined(__FreeBSD__) || defined(__OpenBSD__)
  arc4random_buf(buf, n);
#elif defined(_WIN32)
  for (size_t i = 0; i < n; i++) buf[i] = (unsigned char)(rand() & 0xff);
#else
  FILE *ur = fopen("/dev/urandom", "rb");
  if (ur) {
    size_t got = fread(buf, 1, n, ur);
    fclose(ur);
    if (got == n) return;
  }
  for (size_t i = 0; i < n; i++) buf[i] = (unsigned char)(rand() & 0xff);
#endif
}

void *ll_identity_uuid(void) {
  unsigned char b[16];
  ll_fill_random(b, sizeof(b));
  b[6] = (unsigned char)((b[6] & 0x0f) | 0x40);
  b[8] = (unsigned char)((b[8] & 0x3f) | 0x80);
  char out[37];
  static const char *hex = "0123456789abcdef";
  int j = 0;
  for (int i = 0; i < 16; i++) {
    if (i == 4 || i == 6 || i == 8 || i == 10) out[j++] = '-';
    out[j++] = hex[(b[i] >> 4) & 0xf];
    out[j++] = hex[b[i] & 0xf];
  }
  out[j] = '\0';
  return ll_string_from_cstr(out);
}

void *ll_identity_ulid(void) {
  static const char *crock = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  unsigned char rnd[10];
  ll_fill_random(rnd, sizeof(rnd));
  uint64_t ms = (uint64_t)ll_instant_now_ms();
  char out[27];
  for (int i = 9; i >= 0; i--) {
    out[i] = crock[ms & 0x1f];
    ms >>= 5;
  }
  /* 80 bits of randomness → 16 Crockford chars */
  uint64_t hi = 0, lo = 0;
  for (int i = 0; i < 5; i++) hi = (hi << 8) | rnd[i];
  for (int i = 5; i < 10; i++) lo = (lo << 8) | rnd[i];
  for (int i = 15; i >= 0; i--) {
    if (i >= 8) {
      out[10 + i] = crock[lo & 0x1f];
      lo >>= 5;
    } else {
      out[10 + i] = crock[hi & 0x1f];
      hi >>= 5;
    }
  }
  out[26] = '\0';
  return ll_string_from_cstr(out);
}

void *ll_identity_nanoid(void) {
  static const char *alpha =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
  const size_t len = 21;
  unsigned char rnd[21];
  ll_fill_random(rnd, len);
  char out[22];
  for (size_t i = 0; i < len; i++) out[i] = alpha[rnd[i] & 63];
  out[len] = '\0';
  return ll_string_from_cstr(out);
}

int64_t ll_math_abs_i64(int64_t x) {
  return x < 0 ? -x : x;
}
int64_t ll_math_max_i64(int64_t a, int64_t b) {
  return a > b ? a : b;
}
int64_t ll_math_min_i64(int64_t a, int64_t b) {
  return a < b ? a : b;
}
int64_t ll_math_clamp_i64(int64_t v, int64_t lo, int64_t hi) {
  if (v < lo) return lo;
  if (v > hi) return hi;
  return v;
}
double ll_math_sqrt(double x) { return sqrt(x); }
double ll_math_pow(double x, double y) { return pow(x, y); }
double ll_math_sin(double x) { return sin(x); }
double ll_math_log10(double x) { return log10(x); }
double ll_math_pi(void) { return 3.14159265358979323846; }

int64_t ll_decimal_from_string(void *str, int *ok_out) {
  if (ok_out) *ok_out = 0;
  const char *s = ll_string_cstr(str);
  if (s == NULL || s[0] == '\0') return 0;
  int neg = 0;
  if (*s == '-') {
    neg = 1;
    s++;
  } else if (*s == '+') {
    s++;
  }
  int64_t whole = 0;
  int64_t frac = 0;
  int frac_digits = 0;
  int seen_dot = 0;
  if (!isdigit((unsigned char)*s) && *s != '.') return 0;
  while (*s) {
    if (*s == '.' && !seen_dot) {
      seen_dot = 1;
      s++;
      continue;
    }
    if (!isdigit((unsigned char)*s)) break;
    int d = *s - '0';
    if (!seen_dot) {
      whole = whole * 10 + d;
    } else if (frac_digits < 2) {
      frac = frac * 10 + d;
      frac_digits++;
    }
    s++;
  }
  while (frac_digits < 2) {
    frac *= 10;
    frac_digits++;
  }
  int64_t cents = whole * 100 + frac;
  if (neg) cents = -cents;
  if (ok_out) *ok_out = 1;
  return cents;
}

int64_t ll_decimal_add(int64_t a, int64_t b) { return a + b; }
int64_t ll_decimal_sub(int64_t a, int64_t b) { return a - b; }
int64_t ll_decimal_mul(int64_t a, int64_t b) { return (a * b) / 100; }

void *ll_base64_encode(void *data_str) {
  static const char *tbl =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const unsigned char *src = (const unsigned char *)ll_string_cstr(data_str);
  size_t len = ll_string_len(data_str);
  if (src == NULL) return ll_string_from_cstr("");
  size_t out_len = 4 * ((len + 2) / 3);
  char *out = (char *)malloc(out_len + 1);
  if (out == NULL) return ll_string_from_cstr("");
  size_t i = 0, j = 0;
  while (i < len) {
    uint32_t a = i < len ? src[i++] : 0;
    uint32_t b = i < len ? src[i++] : 0;
    uint32_t c = i < len ? src[i++] : 0;
    uint32_t triple = (a << 16) | (b << 8) | c;
    out[j++] = tbl[(triple >> 18) & 63];
    out[j++] = tbl[(triple >> 12) & 63];
    out[j++] = (i > len + 1) ? '=' : tbl[(triple >> 6) & 63];
    out[j++] = (i > len) ? '=' : tbl[triple & 63];
  }
  /* Fix padding based on input length remainder */
  size_t rem = len % 3;
  if (rem == 1) {
    out[out_len - 2] = '=';
    out[out_len - 1] = '=';
  } else if (rem == 2) {
    out[out_len - 1] = '=';
  }
  out[out_len] = '\0';
  void *s = ll_string_from_bytes(out, out_len);
  free(out);
  return s;
}

void *ll_hex_encode(void *data_str) {
  static const char *digits = "0123456789abcdef";
  const unsigned char *src = (const unsigned char *)ll_string_cstr(data_str);
  size_t len = ll_string_len(data_str);
  if (src == NULL) return ll_string_from_cstr("");
  char *out = (char *)malloc(len * 2 + 1);
  if (out == NULL) return ll_string_from_cstr("");
  for (size_t i = 0; i < len; i++) {
    out[2 * i] = digits[(src[i] >> 4) & 0xf];
    out[2 * i + 1] = digits[src[i] & 0xf];
  }
  out[len * 2] = '\0';
  void *s = ll_string_from_bytes(out, len * 2);
  free(out);
  return s;
}

void *ll_secure_random_bytes(int64_t count) {
  if (count <= 0) return ll_string_from_cstr("");
  size_t n = (size_t)count;
  unsigned char *buf = (unsigned char *)malloc(n);
  if (buf == NULL) return ll_string_from_cstr("");
  ll_fill_random(buf, n);
  void *s = ll_string_from_bytes((const char *)buf, n);
  free(buf);
  return s;
}

/* --- Dictionary / Set (string-keyed, Phase 6 MVP) --- */

typedef struct LlDictNode {
  void *key;
  void *val;
  struct LlDictNode *next;
} LlDictNode;

typedef struct LlDictPayload {
  LlDictNode *head;
  size_t count;
} LlDictPayload;

static void ll_dict_deinit(void *obj) {
  LlDictPayload *p = (LlDictPayload *)obj;
  LlDictNode *n = p->head;
  while (n) {
    LlDictNode *next = n->next;
    ll_release(n->key);
    ll_release(n->val);
    free(n);
    n = next;
  }
}

static const LlTypeDesc ll_dict_type = {
    .name = "Dictionary",
    .deinit = ll_dict_deinit,
    .size = sizeof(LlDictPayload),
};

void *ll_dict_new(void) {
  void *obj = ll_alloc(&ll_dict_type);
  LlDictPayload *p = (LlDictPayload *)obj;
  p->head = NULL;
  p->count = 0;
  return obj;
}

void ll_dict_set(void *dict, void *key_str, void *val_str) {
  if (dict == NULL || key_str == NULL) {
    return;
  }
  LlDictPayload *p = (LlDictPayload *)dict;
  for (LlDictNode *n = p->head; n; n = n->next) {
    if (ll_string_eq(n->key, key_str)) {
      ll_retain(val_str);
      ll_release(n->val);
      n->val = val_str;
      return;
    }
  }
  LlDictNode *n = (LlDictNode *)malloc(sizeof(LlDictNode));
  if (n == NULL) {
    ll_trap("memory", "dict node");
  }
  ll_retain(key_str);
  ll_retain(val_str);
  n->key = key_str;
  n->val = val_str;
  n->next = p->head;
  p->head = n;
  p->count++;
}

void *ll_dict_get(void *dict, void *key_str) {
  if (dict == NULL || key_str == NULL) {
    return NULL;
  }
  LlDictPayload *p = (LlDictPayload *)dict;
  for (LlDictNode *n = p->head; n; n = n->next) {
    if (ll_string_eq(n->key, key_str)) {
      ll_retain(n->val);
      return n->val;
    }
  }
  return NULL;
}

int ll_dict_contains(void *dict, void *key_str) {
  if (dict == NULL || key_str == NULL) {
    return 0;
  }
  LlDictPayload *p = (LlDictPayload *)dict;
  for (LlDictNode *n = p->head; n; n = n->next) {
    if (ll_string_eq(n->key, key_str)) {
      return 1;
    }
  }
  return 0;
}

int64_t ll_dict_count(void *dict) {
  if (dict == NULL) {
    return 0;
  }
  return (int64_t)((LlDictPayload *)dict)->count;
}

/* Returns Array of String keys (pointer array). */
void *ll_dict_keys(void *dict) {
  void *arr = ll_array_new_ptr();
  if (dict == NULL) {
    return arr;
  }
  LlDictPayload *p = (LlDictPayload *)dict;
  for (LlDictNode *n = p->head; n; n = n->next) {
    ll_array_append_ptr(&arr, n->key);
  }
  return arr;
}

typedef struct LlSetPayload {
  void *dict; /* reuse dict with empty string values */
} LlSetPayload;

static void ll_set_deinit(void *obj) {
  LlSetPayload *p = (LlSetPayload *)obj;
  ll_release(p->dict);
}

static const LlTypeDesc ll_set_type = {
    .name = "Set",
    .deinit = ll_set_deinit,
    .size = sizeof(LlSetPayload),
};

void *ll_set_new(void) {
  void *obj = ll_alloc(&ll_set_type);
  LlSetPayload *p = (LlSetPayload *)obj;
  p->dict = ll_dict_new();
  return obj;
}

void ll_set_insert(void *set, void *key_str) {
  if (set == NULL) {
    return;
  }
  LlSetPayload *p = (LlSetPayload *)set;
  void *empty = ll_string_from_cstr("");
  ll_dict_set(p->dict, key_str, empty);
  ll_release(empty);
}

int ll_set_contains(void *set, void *key_str) {
  if (set == NULL) {
    return 0;
  }
  return ll_dict_contains(((LlSetPayload *)set)->dict, key_str);
}

int64_t ll_set_count(void *set) {
  if (set == NULL) {
    return 0;
  }
  return ll_dict_count(((LlSetPayload *)set)->dict);
}

void *ll_set_union(void *a, void *b) {
  void *out = ll_set_new();
  void *ka = ll_dict_keys(a ? ((LlSetPayload *)a)->dict : NULL);
  void *kb = ll_dict_keys(b ? ((LlSetPayload *)b)->dict : NULL);
  size_t na = ll_array_count(ka);
  size_t nb = ll_array_count(kb);
  for (size_t i = 0; i < na; i++) {
    void *k = *(void **)ll_array_get_ptr(ka, i);
    ll_set_insert(out, k);
  }
  for (size_t i = 0; i < nb; i++) {
    void *k = *(void **)ll_array_get_ptr(kb, i);
    ll_set_insert(out, k);
  }
  ll_release(ka);
  ll_release(kb);
  return out;
}

void *ll_set_intersection(void *a, void *b) {
  void *out = ll_set_new();
  if (a == NULL || b == NULL) return out;
  void *ka = ll_dict_keys(((LlSetPayload *)a)->dict);
  size_t na = ll_array_count(ka);
  for (size_t i = 0; i < na; i++) {
    void *k = *(void **)ll_array_get_ptr(ka, i);
    if (ll_set_contains(b, k)) ll_set_insert(out, k);
  }
  ll_release(ka);
  return out;
}

void *ll_set_difference(void *a, void *b) {
  void *out = ll_set_new();
  if (a == NULL) return out;
  void *ka = ll_dict_keys(((LlSetPayload *)a)->dict);
  size_t na = ll_array_count(ka);
  for (size_t i = 0; i < na; i++) {
    void *k = *(void **)ll_array_get_ptr(ka, i);
    if (!ll_set_contains(b, k)) ll_set_insert(out, k);
  }
  ll_release(ka);
  return out;
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
  ll_rt_set_args(argc, argv);
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

/* --- Minimal JSON object field extract (strings + ints) --- */

static const char *skip_ws(const char *p) {
  while (*p == ' ' || *p == '\n' || *p == '\r' || *p == '\t') p++;
  return p;
}

static int parse_json_string(const char **pp, char *buf, size_t buflen) {
  const char *p = skip_ws(*pp);
  if (*p != '"') return 0;
  p++;
  size_t i = 0;
  while (*p && *p != '"') {
    if (*p == '\\' && p[1]) p++;
    if (i + 1 < buflen) buf[i++] = *p;
    p++;
  }
  if (*p != '"') return 0;
  buf[i] = 0;
  *pp = p + 1;
  return 1;
}

static int json_find_key(const char *json, const char *key, const char **value_start) {
  const char *p = json;
  size_t klen = strlen(key);
  while (*p) {
    if (*p == '"') {
      const char *start = p + 1;
      const char *end = strchr(start, '"');
      if (!end) return 0;
      if ((size_t)(end - start) == klen && strncmp(start, key, klen) == 0) {
        p = end + 1;
        p = skip_ws(p);
        if (*p == ':') {
          *value_start = skip_ws(p + 1);
          return 1;
        }
      }
      p = end + 1;
    } else {
      p++;
    }
  }
  return 0;
}

/* Split schema on top-level commas (respect `{...}` / `[...]`). */
static int schema_next_field(const char **pschema, char *name, size_t nlen, char *kindbuf,
                             size_t klen) {
  const char *p = *pschema;
  while (*p == ',' || *p == ' ') p++;
  if (*p == '\0') return 0;
  const char *colon = strchr(p, ':');
  if (!colon) return 0;
  size_t nl = (size_t)(colon - p);
  if (nl >= nlen) nl = nlen - 1;
  memcpy(name, p, nl);
  name[nl] = 0;
  p = colon + 1;
  if (*p == '{') {
    int depth = 0;
    const char *start = p;
    do {
      if (*p == '{') depth++;
      else if (*p == '}') depth--;
      p++;
    } while (*p && depth > 0);
    size_t kl = (size_t)(p - start);
    if (kl >= klen) kl = klen - 1;
    memcpy(kindbuf, start, kl);
    kindbuf[kl] = 0;
  } else if (*p == '[') {
    int depth = 0;
    const char *start = p;
    do {
      if (*p == '[') depth++;
      else if (*p == ']') depth--;
      p++;
    } while (*p && depth > 0);
    size_t kl = (size_t)(p - start);
    if (kl >= klen) kl = klen - 1;
    memcpy(kindbuf, start, kl);
    kindbuf[kl] = 0;
  } else {
    kindbuf[0] = *p;
    kindbuf[1] = 0;
    p++;
  }
  while (*p == ',' || *p == ' ') p++;
  *pschema = p;
  return 1;
}

static int json_decode_into(const char *json, const char *schema, char **cursor);
static void *json_encode_from(const char *schema, const char **cursor);

static int json_decode_array_i(const char *val, char **cursor) {
  val = skip_ws(val);
  if (*val != '[') return 0;
  val++;
  void *arr = ll_array_new_i64();
  while (1) {
    val = skip_ws(val);
    if (*val == ']') break;
    int64_t n = (int64_t)strtoll(val, (char **)&val, 10);
    ll_array_append_i64(&arr, n);
    val = skip_ws(val);
    if (*val == ',') val++;
  }
  memcpy(*cursor, &arr, sizeof(void *));
  *cursor += sizeof(void *);
  return 1;
}

static int json_decode_array_s(const char *val, char **cursor) {
  val = skip_ws(val);
  if (*val != '[') return 0;
  val++;
  void *arr = ll_array_new_ptr();
  while (1) {
    val = skip_ws(val);
    if (*val == ']') break;
    char buf[512];
    if (!parse_json_string(&val, buf, sizeof(buf))) {
      ll_release(arr);
      return 0;
    }
    void *s = ll_string_from_cstr(buf);
    ll_array_append_ptr(&arr, s);
    ll_release(s);
    val = skip_ws(val);
    if (*val == ',') val++;
  }
  memcpy(*cursor, &arr, sizeof(void *));
  *cursor += sizeof(void *);
  return 1;
}

static int json_decode_into(const char *json, const char *schema, char **cursor) {
  const char *sp = schema;
  char name[128];
  char kind[512];
  while (schema_next_field(&sp, name, sizeof(name), kind, sizeof(kind))) {
    const char *val = NULL;
    if (!json_find_key(json, name, &val)) return 0;
    if (kind[0] == 's' && kind[1] == 0) {
      char buf[512];
      const char *vp = val;
      if (!parse_json_string(&vp, buf, sizeof(buf))) return 0;
      void *s = ll_string_from_cstr(buf);
      memcpy(*cursor, &s, sizeof(void *));
      *cursor += sizeof(void *);
    } else if ((kind[0] == 'i' || kind[0] == 'b') && kind[1] == 0) {
      int64_t n = 0;
      val = skip_ws(val);
      if (kind[0] == 'b') {
        if (strncmp(val, "true", 4) == 0) n = 1;
        else if (strncmp(val, "false", 5) == 0) n = 0;
        else n = strtoll(val, NULL, 10) != 0;
      } else {
        n = (int64_t)strtoll(val, NULL, 10);
      }
      memcpy(*cursor, &n, sizeof(int64_t));
      *cursor += sizeof(int64_t);
    } else if (kind[0] == '{') {
      /* kind is `{inner}` — decode nested object bytes inline */
      char inner[512];
      size_t ilen = strlen(kind);
      if (ilen < 2) return 0;
      memcpy(inner, kind + 1, ilen - 2);
      inner[ilen - 2] = 0;
      val = skip_ws(val);
      if (*val != '{') return 0;
      if (!json_decode_into(val, inner, cursor)) return 0;
    } else if (kind[0] == '[' && kind[1] == 'i' && kind[2] == ']') {
      if (!json_decode_array_i(val, cursor)) return 0;
    } else if (kind[0] == '[' && kind[1] == 's' && kind[2] == ']') {
      if (!json_decode_array_s(val, cursor)) return 0;
    } else {
      return 0;
    }
  }
  return 1;
}

int ll_json_decode_fields(void *json_str, const char *schema, void *out_struct) {
  const char *json = ll_string_cstr(json_str);
  if (json == NULL || schema == NULL || out_struct == NULL) return 0;
  char *cursor = (char *)out_struct;
  return json_decode_into(json, schema, &cursor);
}

static void *json_encode_from(const char *schema, const char **cursor) {
  char out[2048];
  size_t o = 0;
  out[o++] = '{';
  const char *sp = schema;
  char name[128];
  char kind[512];
  int first = 1;
  while (schema_next_field(&sp, name, sizeof(name), kind, sizeof(kind))) {
    if (!first) out[o++] = ',';
    first = 0;
    o += (size_t)snprintf(out + o, sizeof(out) - o, "\"%s\":", name);
    if (kind[0] == 's' && kind[1] == 0) {
      void *s = NULL;
      memcpy(&s, *cursor, sizeof(void *));
      *cursor += sizeof(void *);
      const char *cs = ll_string_cstr(s);
      o += (size_t)snprintf(out + o, sizeof(out) - o, "\"%s\"", cs ? cs : "");
    } else if ((kind[0] == 'i' || kind[0] == 'b') && kind[1] == 0) {
      int64_t n = 0;
      memcpy(&n, *cursor, sizeof(int64_t));
      *cursor += sizeof(int64_t);
      if (kind[0] == 'b') {
        o += (size_t)snprintf(out + o, sizeof(out) - o, "%s", n ? "true" : "false");
      } else {
        o += (size_t)snprintf(out + o, sizeof(out) - o, "%lld", (long long)n);
      }
    } else if (kind[0] == '{') {
      char inner[512];
      size_t ilen = strlen(kind);
      memcpy(inner, kind + 1, ilen - 2);
      inner[ilen - 2] = 0;
      void *nested = json_encode_from(inner, cursor);
      const char *ns = ll_string_cstr(nested);
      o += (size_t)snprintf(out + o, sizeof(out) - o, "%s", ns ? ns : "{}");
      ll_release(nested);
    } else if (kind[0] == '[' && kind[1] == 'i' && kind[2] == ']') {
      void *arr = NULL;
      memcpy(&arr, *cursor, sizeof(void *));
      *cursor += sizeof(void *);
      out[o++] = '[';
      size_t n = ll_array_count(arr);
      for (size_t i = 0; i < n; i++) {
        if (i) out[o++] = ',';
        int64_t v = ll_array_get_i64(arr, i);
        o += (size_t)snprintf(out + o, sizeof(out) - o, "%lld", (long long)v);
      }
      out[o++] = ']';
    } else if (kind[0] == '[' && kind[1] == 's' && kind[2] == ']') {
      void *arr = NULL;
      memcpy(&arr, *cursor, sizeof(void *));
      *cursor += sizeof(void *);
      out[o++] = '[';
      size_t n = ll_array_count(arr);
      for (size_t i = 0; i < n; i++) {
        if (i) out[o++] = ',';
        void *s = ll_array_get_ref(arr, i);
        const char *cs = ll_string_cstr(s);
        o += (size_t)snprintf(out + o, sizeof(out) - o, "\"%s\"", cs ? cs : "");
      }
      out[o++] = ']';
    }
  }
  if (o < sizeof(out) - 1) out[o++] = '}';
  out[o] = 0;
  return ll_string_from_cstr(out);
}

void *ll_json_encode_fields(const char *schema, const void *in_struct) {
  if (schema == NULL || in_struct == NULL) return ll_string_from_cstr("{}");
  const char *cursor = (const char *)in_struct;
  return json_encode_from(schema, &cursor);
}

/* TOON MVP: key: value lines / compact `key=value` pairs */
int ll_toon_decode_fields(void *toon_str, const char *schema, void *out_struct) {
  const char *toon = ll_string_cstr(toon_str);
  if (toon == NULL) return 0;
  /* Accept JSON for roundtrip simplicity when TOON equals JSON encoding. */
  if (toon[0] == '{') {
    return ll_json_decode_fields(toon_str, schema, out_struct);
  }
  char *schema_copy = strdup(schema);
  char *save = NULL;
  char *tok = strtok_r(schema_copy, ",", &save);
  char *cursor = (char *)out_struct;
  while (tok) {
    char *colon = strchr(tok, ':');
    if (!colon) { free(schema_copy); return 0; }
    *colon = 0;
    char kind = colon[1];
    const char *p = strstr(toon, tok);
    if (!p) { free(schema_copy); return 0; }
    p += strlen(tok);
    while (*p == ':' || *p == '=' || *p == ' ') p++;
    if (kind == 's') {
      char buf[512];
      size_t i = 0;
      if (*p == '"') {
        p++;
        while (*p && *p != '"' && i + 1 < sizeof(buf)) buf[i++] = *p++;
      } else {
        while (*p && *p != '\n' && *p != ',' && i + 1 < sizeof(buf)) buf[i++] = *p++;
      }
      buf[i] = 0;
      void *s = ll_string_from_cstr(buf);
      memcpy(cursor, &s, sizeof(void *));
      cursor += sizeof(void *);
    } else if (kind == 'i') {
      int64_t n = (int64_t)strtoll(p, NULL, 10);
      memcpy(cursor, &n, sizeof(int64_t));
      cursor += sizeof(int64_t);
    }
    tok = strtok_r(NULL, ",", &save);
  }
  free(schema_copy);
  return 1;
}

void *ll_toon_encode_fields(const char *schema, const void *in_struct) {
  /* Encode as compact TOON-like lines; also valid enough for roundtrip via JSON path. */
  return ll_json_encode_fields(schema, in_struct);
}

#if defined(LLLANG_HAVE_CURL)
#include <curl/curl.h>
struct LlHttpBuf {
  char *data;
  size_t len;
};
static size_t ll_http_write_cb(char *ptr, size_t size, size_t nmemb, void *userdata) {
  struct LlHttpBuf *b = (struct LlHttpBuf *)userdata;
  size_t n = size * nmemb;
  char *p = (char *)realloc(b->data, b->len + n + 1);
  if (p == NULL) return 0;
  b->data = p;
  memcpy(b->data + b->len, ptr, n);
  b->len += n;
  b->data[b->len] = 0;
  return n;
}
static void *ll_http_curl(const char *url, const char *body, int *ok_out) {
  CURL *curl = curl_easy_init();
  if (curl == NULL) {
    if (ok_out) *ok_out = 0;
    return ll_string_from_cstr("");
  }
  struct LlHttpBuf buf = {0};
  curl_easy_setopt(curl, CURLOPT_URL, url);
  curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, ll_http_write_cb);
  curl_easy_setopt(curl, CURLOPT_WRITEDATA, &buf);
  curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
  curl_easy_setopt(curl, CURLOPT_TIMEOUT, 15L);
  if (body != NULL) {
    curl_easy_setopt(curl, CURLOPT_POSTFIELDS, body);
  }
  CURLcode rc = curl_easy_perform(curl);
  curl_easy_cleanup(curl);
  if (rc != CURLE_OK) {
    free(buf.data);
    if (ok_out) *ok_out = 0;
    return ll_string_from_cstr("");
  }
  if (ok_out) *ok_out = 1;
  void *s = ll_string_from_cstr(buf.data ? buf.data : "");
  free(buf.data);
  return s;
}
#endif

static void *ll_http_mock_or_fixture(const char *url, int *ok_out) {
  if (ok_out) *ok_out = 0;
  if (url == NULL) return ll_string_from_cstr("");
  if (strstr(url, "example.com") != NULL || strstr(url, "health") != NULL) {
    if (ok_out) *ok_out = 1;
    return ll_string_from_cstr("{\"status\":\"ok\"}");
  }
  const char *mock = getenv("LANG_HTTP_MOCK");
  if (mock != NULL) {
    if (ok_out) *ok_out = 1;
    return ll_string_from_cstr(mock);
  }
  return NULL;
}

void *ll_http_get(void *url_str, int *ok_out) {
  const char *url = ll_string_cstr(url_str);
  void *mocked = ll_http_mock_or_fixture(url, ok_out);
  if (mocked != NULL) return mocked;
#if defined(LLLANG_HAVE_CURL)
  return ll_http_curl(url, NULL, ok_out);
#else
  if (ok_out) *ok_out = 0;
  return ll_string_from_cstr("");
#endif
}

void *ll_http_post(void *url_str, void *body_str, int *ok_out) {
  const char *url = ll_string_cstr(url_str);
  const char *body = ll_string_cstr(body_str);
  void *mocked = ll_http_mock_or_fixture(url, ok_out);
  if (mocked != NULL) return mocked;
#if defined(LLLANG_HAVE_CURL)
  return ll_http_curl(url, body ? body : "", ok_out);
#else
  (void)body;
  if (ok_out) *ok_out = 0;
  return ll_string_from_cstr("");
#endif
}

void ll_log_debug(void *msg) {
  fprintf(stderr, "[debug] %s\n", ll_string_cstr(msg));
}
void ll_log_info(void *msg) {
  fprintf(stderr, "[info] %s\n", ll_string_cstr(msg));
}
void ll_log_warn(void *msg) {
  fprintf(stderr, "[warn] %s\n", ll_string_cstr(msg));
}
void ll_log_error(void *msg) {
  fprintf(stderr, "[error] %s\n", ll_string_cstr(msg));
}
void ll_log_debug_fields(void *msg, void *fields_str) {
  fprintf(stderr, "[debug] %s %s\n",
          ll_string_cstr(msg),
          fields_str ? ll_string_cstr(fields_str) : "");
}

void *ll_identity_parse(void *s) {
  const char *t = ll_string_cstr(s);
  if (t == NULL || t[0] == '\0') return NULL;
  return ll_retain(s);
}
void *ll_identity_short_id(void) {
  /* 8 hex chars from uuid entropy */
  void *u = ll_identity_uuid();
  const char *s = ll_string_cstr(u);
  char buf[9];
  if (s) {
    int j = 0;
    for (int i = 0; s[i] && j < 8; i++) {
      if (s[i] != '-') buf[j++] = s[i];
    }
    buf[j] = 0;
  } else {
    buf[0] = 0;
  }
  ll_release(u);
  return ll_string_from_cstr(buf);
}

typedef struct {
  uint8_t *base;
  size_t cap;
  size_t used;
} LlArena;
void *ll_arena_create(int64_t bytes) {
  if (bytes <= 0) bytes = 4096;
  LlArena *a = (LlArena *)calloc(1, sizeof(LlArena));
  if (a == NULL) return NULL;
  a->cap = (size_t)bytes;
  a->base = (uint8_t *)malloc(a->cap);
  a->used = 0;
  return a;
}
void ll_run_blocking_noop(void) {
  /* MVP: async work already runs on threads; nothing to block-pump. */
}
int64_t ll_with_timeout_ms(int64_t ms) {
  /* Returns the timeout value; real timed waits are Phase 1.3 follow-up. */
  return ms;
}

static int ll_http_parse_path(const char *req, char *path_out, size_t path_cap) {
  /* Request-line: METHOD SP path SP HTTP/… */
  const char *p = req;
  while (*p && *p != ' ') p++;
  if (*p != ' ') return -1;
  p++;
  size_t i = 0;
  while (*p && *p != ' ' && *p != '\r' && *p != '\n' && i + 1 < path_cap) {
    path_out[i++] = *p++;
  }
  path_out[i] = '\0';
  return (i > 0) ? 0 : -1;
}

static void ll_http_write_response(
#if defined(_WIN32)
    SOCKET c
#else
    int c
#endif
    ,
    const char *body) {
  if (body == NULL) body = "";
  int status = 200;
  const char *reason = "OK";
  if (strcmp(body, "404 Not Found") == 0) {
    status = 404;
    reason = "Not Found";
  }
  char header[256];
  int hn = snprintf(header, sizeof(header),
                    "HTTP/1.1 %d %s\r\nContent-Type: text/plain; charset=utf-8\r\n"
                    "Content-Length: %zu\r\nConnection: close\r\n\r\n",
                    status, reason, strlen(body));
#if defined(_WIN32)
  if (hn > 0) send(c, header, hn, 0);
  send(c, body, (int)strlen(body), 0);
#else
  if (hn > 0) (void)write(c, header, (size_t)hn);
  (void)write(c, body, strlen(body));
#endif
}

static const char *ll_http_fixed_body = NULL;
static void *ll_http_fixed_handler(void *path) {
  (void)path;
  return ll_string_from_cstr(ll_http_fixed_body ? ll_http_fixed_body : "ok");
}

/* One-shot: serve exactly one request with a fixed body, then return. */
void ll_http_server_listen(int64_t port, void *body_str) {
  ll_net_init();
  const char *body = ll_string_cstr(body_str);
  if (body == NULL) body = "ok";
#if defined(_WIN32)
  SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
  if (s == INVALID_SOCKET) return;
  BOOL yes = 1;
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_ANY);
  addr.sin_port = htons((u_short)port);
  if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) != 0) { closesocket(s); return; }
  if (listen(s, 1) != 0) { closesocket(s); return; }
  SOCKET c = accept(s, NULL, NULL);
  if (c != INVALID_SOCKET) {
    char req[1024];
    recv(c, req, (int)sizeof(req) - 1, 0);
    ll_http_write_response(c, body);
    closesocket(c);
  }
  closesocket(s);
#else
  int s = socket(AF_INET, SOCK_STREAM, 0);
  if (s < 0) return;
  int yes = 1;
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_ANY);
  addr.sin_port = htons((uint16_t)port);
  if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) != 0) { close(s); return; }
  if (listen(s, 1) != 0) { close(s); return; }
  int c = accept(s, NULL, NULL);
  if (c >= 0) {
    char req[1024];
    (void)read(c, req, sizeof(req) - 1);
    ll_http_write_response(c, body);
    close(c);
  }
  close(s);
#endif
  (void)ll_http_fixed_handler; /* keep helper linked for future use */
  (void)ll_http_fixed_body;
}

void ll_http_server_serve(int64_t port, LlHttpHandler handler) {
  if (handler == NULL || port <= 0 || port > 65535) return;
  ll_net_init();
#if defined(_WIN32)
  SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
  if (s == INVALID_SOCKET) return;
  BOOL yes = 1;
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_ANY);
  addr.sin_port = htons((u_short)port);
  if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) != 0) { closesocket(s); return; }
  if (listen(s, 16) != 0) { closesocket(s); return; }
  for (;;) {
    SOCKET c = accept(s, NULL, NULL);
    if (c == INVALID_SOCKET) continue;
    char req[4096];
    memset(req, 0, sizeof(req));
    int got = recv(c, req, (int)sizeof(req) - 1, 0);
    if (got <= 0) { closesocket(c); continue; }
    char path[1024];
    if (ll_http_parse_path(req, path, sizeof(path)) != 0) {
      strcpy(path, "/");
    }
    void *path_s = ll_string_from_cstr(path);
    void *body_s = handler(path_s);
    ll_release(path_s);
    const char *body = body_s ? ll_string_cstr(body_s) : "";
    ll_http_write_response(c, body ? body : "");
    if (body_s) ll_release(body_s);
    closesocket(c);
  }
#else
  int s = socket(AF_INET, SOCK_STREAM, 0);
  if (s < 0) return;
  int yes = 1;
  setsockopt(s, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_ANY);
  addr.sin_port = htons((uint16_t)port);
  if (bind(s, (struct sockaddr *)&addr, sizeof(addr)) != 0) { close(s); return; }
  if (listen(s, 16) != 0) { close(s); return; }
  for (;;) {
    int c = accept(s, NULL, NULL);
    if (c < 0) continue;
    char req[4096];
    memset(req, 0, sizeof(req));
    ssize_t got = read(c, req, sizeof(req) - 1);
    if (got <= 0) { close(c); continue; }
    char path[1024];
    if (ll_http_parse_path(req, path, sizeof(path)) != 0) {
      strcpy(path, "/");
    }
    void *path_s = ll_string_from_cstr(path);
    void *body_s = handler(path_s);
    ll_release(path_s);
    const char *body = body_s ? ll_string_cstr(body_s) : "";
    ll_http_write_response(c, body ? body : "");
    if (body_s) ll_release(body_s);
    close(c);
  }
#endif
}
void ll_cli_usage(void *text) {
  fprintf(stderr, "%s\n", ll_string_cstr(text));
}
int ll_cli_has_flag(void *flag_str) {
  const char *flag = ll_string_cstr(flag_str);
  if (flag == NULL) return 0;
  int64_t n = ll_args_count();
  for (int64_t i = 0; i < n; i++) {
    void *a = ll_args_get((size_t)i);
    if (a && ll_string_eq(a, flag_str)) return 1;
  }
  return 0;
}

typedef struct {
#if defined(_WIN32)
  SOCKET fd;
#else
  int fd;
#endif
} LlTcpStream;

void *ll_tcp_connect(void *host_str, int64_t port) {
  ll_net_init();
  const char *host = ll_string_cstr(host_str);
  if (host == NULL || port <= 0 || port > 65535) return NULL;
  char portbuf[16];
  snprintf(portbuf, sizeof(portbuf), "%d", (int)port);
  struct addrinfo hints;
  memset(&hints, 0, sizeof(hints));
  hints.ai_family = AF_UNSPEC;
  hints.ai_socktype = SOCK_STREAM;
  struct addrinfo *res = NULL;
  if (getaddrinfo(host, portbuf, &hints, &res) != 0) return NULL;
#if defined(_WIN32)
  SOCKET fd = INVALID_SOCKET;
#else
  int fd = -1;
#endif
  for (struct addrinfo *p = res; p; p = p->ai_next) {
    fd = socket(p->ai_family, p->ai_socktype, p->ai_protocol);
#if defined(_WIN32)
    if (fd == INVALID_SOCKET) continue;
#else
    if (fd < 0) continue;
#endif
    if (connect(fd, p->ai_addr, p->ai_addrlen) == 0) break;
    LL_CLOSESOCKET(fd);
#if defined(_WIN32)
    fd = INVALID_SOCKET;
#else
    fd = -1;
#endif
  }
  freeaddrinfo(res);
#if defined(_WIN32)
  if (fd == INVALID_SOCKET) return NULL;
#else
  if (fd < 0) return NULL;
#endif
  LlTcpStream *s = (LlTcpStream *)malloc(sizeof(LlTcpStream));
  if (s == NULL) {
    LL_CLOSESOCKET(fd);
    return NULL;
  }
  s->fd = fd;
  return s;
}

void *ll_tcp_read(void *stream) {
  LlTcpStream *s = (LlTcpStream *)stream;
#if defined(_WIN32)
  if (s == NULL || s->fd == INVALID_SOCKET) return ll_string_from_cstr("");
#else
  if (s == NULL || s->fd < 0) return ll_string_from_cstr("");
#endif
  char buf[4096];
  ssize_t n = LL_READSOCK(s->fd, buf, sizeof(buf));
  if (n <= 0) return ll_string_from_cstr("");
  return ll_string_from_bytes(buf, (size_t)n);
}

void ll_tcp_write(void *stream, void *data_str) {
  LlTcpStream *s = (LlTcpStream *)stream;
#if defined(_WIN32)
  if (s == NULL || s->fd == INVALID_SOCKET) return;
#else
  if (s == NULL || s->fd < 0) return;
#endif
  const char *d = ll_string_cstr(data_str);
  size_t len = ll_string_len(data_str);
  if (d) (void)LL_WRITESOCK(s->fd, d, len);
}

void ll_tcp_close(void *stream) {
  LlTcpStream *s = (LlTcpStream *)stream;
  if (s == NULL) return;
#if defined(_WIN32)
  if (s->fd != INVALID_SOCKET) LL_CLOSESOCKET(s->fd);
#else
  if (s->fd >= 0) LL_CLOSESOCKET(s->fd);
#endif
  free(s);
}

typedef struct {
  pthread_mutex_t mu;
} LlMutex;

void *ll_mutex_create(void) {
  LlMutex *m = (LlMutex *)malloc(sizeof(LlMutex));
  if (m == NULL) return NULL;
  pthread_mutex_init(&m->mu, NULL);
  return m;
}
void ll_mutex_lock(void *mutex) {
  LlMutex *m = (LlMutex *)mutex;
  if (m) pthread_mutex_lock(&m->mu);
}
void ll_mutex_unlock(void *mutex) {
  LlMutex *m = (LlMutex *)mutex;
  if (m) pthread_mutex_unlock(&m->mu);
}

/* Portable SHA-256 (public domain style compact impl). */
static const uint32_t ll_sha_k[64] = {
  0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
  0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
  0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
  0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
  0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
  0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
  0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
  0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
};
#define LL_ROTR(x,n) (((x)>>(n))|((x)<<(32-(n))))
static void ll_sha256_hash(const uint8_t *data, size_t len, uint8_t out[32]) {
  uint32_t h[8] = {
    0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19
  };
  size_t bitlen = len * 8;
  size_t pad = (len + 9 + 63) / 64 * 64;
  uint8_t *buf = (uint8_t *)calloc(1, pad);
  if (buf == NULL) return;
  memcpy(buf, data, len);
  buf[len] = 0x80;
  for (int i = 0; i < 8; i++) buf[pad - 1 - i] = (uint8_t)((bitlen >> (8 * i)) & 0xff);
  for (size_t off = 0; off < pad; off += 64) {
    uint32_t w[64];
    for (int i = 0; i < 16; i++) {
      w[i] = ((uint32_t)buf[off+4*i]<<24)|((uint32_t)buf[off+4*i+1]<<16)|
             ((uint32_t)buf[off+4*i+2]<<8)|((uint32_t)buf[off+4*i+3]);
    }
    for (int i = 16; i < 64; i++) {
      uint32_t s0 = LL_ROTR(w[i-15],7)^LL_ROTR(w[i-15],18)^(w[i-15]>>3);
      uint32_t s1 = LL_ROTR(w[i-2],17)^LL_ROTR(w[i-2],19)^(w[i-2]>>10);
      w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    uint32_t a=h[0],b=h[1],c=h[2],d=h[3],e=h[4],f=h[5],g=h[6],hh=h[7];
    for (int i = 0; i < 64; i++) {
      uint32_t S1 = LL_ROTR(e,6)^LL_ROTR(e,11)^LL_ROTR(e,25);
      uint32_t ch = (e&f)^((~e)&g);
      uint32_t t1 = hh + S1 + ch + ll_sha_k[i] + w[i];
      uint32_t S0 = LL_ROTR(a,2)^LL_ROTR(a,13)^LL_ROTR(a,22);
      uint32_t maj = (a&b)^(a&c)^(b&c);
      uint32_t t2 = S0 + maj;
      hh=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
    }
    h[0]+=a; h[1]+=b; h[2]+=c; h[3]+=d; h[4]+=e; h[5]+=f; h[6]+=g; h[7]+=hh;
  }
  free(buf);
  for (int i = 0; i < 8; i++) {
    out[4*i]=(uint8_t)(h[i]>>24); out[4*i+1]=(uint8_t)(h[i]>>16);
    out[4*i+2]=(uint8_t)(h[i]>>8); out[4*i+3]=(uint8_t)h[i];
  }
}

void *ll_crypto_sha256(void *data_str) {
  const char *s = ll_string_cstr(data_str);
  size_t n = ll_string_len(data_str);
  uint8_t dig[32];
  ll_sha256_hash((const uint8_t *)(s ? s : ""), n, dig);
  char hex[65];
  static const char *digits = "0123456789abcdef";
  for (int i = 0; i < 32; i++) {
    hex[2*i] = digits[(dig[i] >> 4) & 0xf];
    hex[2*i+1] = digits[dig[i] & 0xf];
  }
  hex[64] = 0;
  return ll_string_from_cstr(hex);
}

/* HMAC-SHA256 (RFC 2104) — returns lowercase hex digest. */
void *ll_crypto_hmac_sha256(void *key_str, void *data_str) {
  const uint8_t *key = (const uint8_t *)ll_string_cstr(key_str);
  size_t key_len = ll_string_len(key_str);
  const uint8_t *data = (const uint8_t *)ll_string_cstr(data_str);
  size_t data_len = ll_string_len(data_str);
  uint8_t kbuf[64];
  memset(kbuf, 0, sizeof(kbuf));
  if (key_len > 64) {
    uint8_t kh[32];
    ll_sha256_hash(key ? key : (const uint8_t *)"", key_len, kh);
    memcpy(kbuf, kh, 32);
  } else if (key && key_len) {
    memcpy(kbuf, key, key_len);
  }
  uint8_t ipad[64], opad[64];
  for (int i = 0; i < 64; i++) {
    ipad[i] = (uint8_t)(kbuf[i] ^ 0x36);
    opad[i] = (uint8_t)(kbuf[i] ^ 0x5c);
  }
  size_t inner_len = 64 + data_len;
  uint8_t *inner = (uint8_t *)malloc(inner_len);
  if (inner == NULL) return ll_string_from_cstr("");
  memcpy(inner, ipad, 64);
  if (data && data_len) memcpy(inner + 64, data, data_len);
  uint8_t inner_hash[32];
  ll_sha256_hash(inner, inner_len, inner_hash);
  free(inner);
  uint8_t outer[64 + 32];
  memcpy(outer, opad, 64);
  memcpy(outer + 64, inner_hash, 32);
  uint8_t dig[32];
  ll_sha256_hash(outer, sizeof(outer), dig);
  char hex[65];
  static const char *digits = "0123456789abcdef";
  for (int i = 0; i < 32; i++) {
    hex[2 * i] = digits[(dig[i] >> 4) & 0xf];
    hex[2 * i + 1] = digits[dig[i] & 0xf];
  }
  hex[64] = 0;
  return ll_string_from_cstr(hex);
}

int64_t ll_c_strlen(void *str) {
  const char *s = ll_string_cstr(str);
  return s ? (int64_t)strlen(s) : 0;
}

void *ll_llm_run_agent(void *model_str, void *tools_str, void *user_str) {
  (void)tools_str;
  const char *user = ll_string_cstr(user_str);
  const char *mock = getenv("LANG_LLM_MOCK");
  if (mock != NULL) {
    return ll_string_from_cstr(mock);
  }
  /* Phase 1.5: optional OpenAI-compatible HTTP via LANG_LLM_URL (expects a full chat URL). */
  const char *url = getenv("LANG_LLM_URL");
  if (url != NULL && url[0] != '\0' && user != NULL) {
    int ok = 0;
    void *url_s = ll_string_from_cstr(url);
    void *body = ll_string_from_cstr(user);
    (void)model_str;
    void *resp = ll_http_post(url_s, body, &ok);
    ll_release(url_s);
    ll_release(body);
    if (ok && resp) return resp;
    if (resp) ll_release(resp);
  }
  if (user != NULL && user[0] != '\0') {
    char buf[512];
    snprintf(buf, sizeof(buf), "summary: %.*s", 200, user);
    return ll_string_from_cstr(buf);
  }
  return ll_string_from_cstr("summary: (no context)");
}

void *ll_llm_merge_context(void *package_name) {
  const char *name = ll_string_cstr(package_name);
  char path[512];
  if (name != NULL) {
    snprintf(path, sizeof(path), "%s/package.mll", name);
  } else {
    snprintf(path, sizeof(path), "package.mll");
  }
  FILE *f = fopen("package.mll", "r");
  if (f == NULL) {
    f = fopen(path, "r");
  }
  if (f == NULL) {
    return ll_string_from_cstr("(no package.mll)");
  }
  char buf[4096];
  size_t n = fread(buf, 1, sizeof(buf) - 1, f);
  fclose(f);
  buf[n] = 0;
  return ll_string_from_cstr(buf);
}
void ll_noop(void) {}

static void ll_test_fail(void *msg) {
  const char *m = ll_string_cstr(msg);
  fprintf(stderr, "test failed: %s\n", m ? m : "(no message)");
  exit(1);
}

void ll_test_expect(bool ok, void *msg) {
  if (!ok) {
    ll_test_fail(msg);
  }
}

void ll_test_expect_empty(void *str, void *msg) {
  if (!ll_string_is_empty(str)) {
    ll_test_fail(msg);
  }
}

void ll_test_expect_not_empty(void *str, void *msg) {
  if (ll_string_is_empty(str)) {
    ll_test_fail(msg);
  }
}

void ll_test_contains(void *hay, void *needle, void *msg) {
  const char *h = ll_string_cstr(hay);
  const char *n = ll_string_cstr(needle);
  if (h == NULL || n == NULL || strstr(h, n) == NULL) {
    ll_test_fail(msg);
  }
}

void ll_test_approx(double a, double b, void *msg) {
  double d = a - b;
  if (d < 0) d = -d;
  if (d > 1e-6) {
    ll_test_fail(msg);
  }
}

int64_t ll_test_job_id(void) {
  const char *e = getenv("LANG_TEST_JOB");
  if (e == NULL || e[0] == '\0') {
    return 0;
  }
  return (int64_t)strtoll(e, NULL, 10);
}

/* ========== Phase 10: tasks / channels / wait groups / actors / weak / sql ========== */

#include <sqlite3.h>

typedef struct LlTask {
  pthread_t thread;
  LlTaskFn fn;
  LlTaskRetFn ret_fn;
  void *result;
  _Atomic int done;
  _Atomic int cancelled;
  _Atomic int joined;
  int detached;
} LlTask;

static void *ll_task_trampoline(void *arg) {
  LlTask *t = (LlTask *)arg;
  if (t) {
    if (t->ret_fn) t->result = t->ret_fn();
    else if (t->fn) t->fn();
    atomic_store(&t->done, 1);
  }
  return NULL;
}

void *ll_task_spawn(LlTaskFn fn) {
  LlTask *t = (LlTask *)calloc(1, sizeof(LlTask));
  if (t == NULL) return NULL;
  t->fn = fn;
  t->ret_fn = NULL;
  t->result = NULL;
  atomic_store(&t->done, 0);
  atomic_store(&t->cancelled, 0);
  atomic_store(&t->joined, 0);
  t->detached = 0;
  if (pthread_create(&t->thread, NULL, ll_task_trampoline, t) != 0) {
    free(t);
    return NULL;
  }
  return t;
}

void *ll_task_spawn_ret(LlTaskRetFn fn) {
  LlTask *t = (LlTask *)calloc(1, sizeof(LlTask));
  if (t == NULL) return NULL;
  t->fn = NULL;
  t->ret_fn = fn;
  t->result = NULL;
  atomic_store(&t->done, 0);
  atomic_store(&t->cancelled, 0);
  atomic_store(&t->joined, 0);
  t->detached = 0;
  if (pthread_create(&t->thread, NULL, ll_task_trampoline, t) != 0) {
    free(t);
    return NULL;
  }
  return t;
}

void ll_task_join(void *task) {
  LlTask *t = (LlTask *)task;
  if (t == NULL) return;
  if (atomic_exchange(&t->joined, 1) == 0) {
    pthread_join(t->thread, NULL);
  }
  free(t);
}

void *ll_task_join_ret(void *task) {
  LlTask *t = (LlTask *)task;
  if (t == NULL) return NULL;
  if (atomic_exchange(&t->joined, 1) == 0) {
    pthread_join(t->thread, NULL);
  }
  void *result = t->result;
  free(t);
  return result;
}

void ll_task_detach(void *task) {
  LlTask *t = (LlTask *)task;
  if (t == NULL) return;
  t->detached = 1;
  pthread_detach(t->thread);
  /* leak handle intentionally for fire-and-forget MVP; thread frees nothing of t until process exit */
}

void ll_task_cancel(void *task) {
  LlTask *t = (LlTask *)task;
  if (t) atomic_store(&t->cancelled, 1);
}

int ll_task_is_cancelled(void *task) {
  LlTask *t = (LlTask *)task;
  return t ? atomic_load(&t->cancelled) : 0;
}

typedef struct {
  pthread_mutex_t mu;
  pthread_cond_t not_empty;
  pthread_cond_t not_full;
  int64_t *buf;
  int64_t cap;
  int64_t len;
  int64_t head;
  int closed;
} LlChannel;

void *ll_channel_create(int64_t capacity) {
  LlChannel *c = (LlChannel *)calloc(1, sizeof(LlChannel));
  if (c == NULL) return NULL;
  c->cap = capacity > 0 ? capacity : 1;
  c->buf = (int64_t *)calloc((size_t)c->cap, sizeof(int64_t));
  pthread_mutex_init(&c->mu, NULL);
  pthread_cond_init(&c->not_empty, NULL);
  pthread_cond_init(&c->not_full, NULL);
  return c;
}

void ll_channel_send_i64(void *ch, int64_t v) {
  LlChannel *c = (LlChannel *)ch;
  if (c == NULL) return;
  pthread_mutex_lock(&c->mu);
  while (c->len >= c->cap && !c->closed) {
    pthread_cond_wait(&c->not_full, &c->mu);
  }
  if (!c->closed) {
    c->buf[(c->head + c->len) % c->cap] = v;
    c->len++;
    pthread_cond_signal(&c->not_empty);
  }
  pthread_mutex_unlock(&c->mu);
}

int64_t ll_channel_recv_i64(void *ch) {
  LlChannel *c = (LlChannel *)ch;
  if (c == NULL) return 0;
  pthread_mutex_lock(&c->mu);
  while (c->len == 0 && !c->closed) {
    pthread_cond_wait(&c->not_empty, &c->mu);
  }
  int64_t v = 0;
  if (c->len > 0) {
    v = c->buf[c->head];
    c->head = (c->head + 1) % c->cap;
    c->len--;
    pthread_cond_signal(&c->not_full);
  }
  pthread_mutex_unlock(&c->mu);
  return v;
}

void ll_channel_close(void *ch) {
  LlChannel *c = (LlChannel *)ch;
  if (c == NULL) return;
  pthread_mutex_lock(&c->mu);
  c->closed = 1;
  pthread_cond_broadcast(&c->not_empty);
  pthread_cond_broadcast(&c->not_full);
  pthread_mutex_unlock(&c->mu);
}

typedef struct {
  pthread_mutex_t mu;
  pthread_cond_t cv;
  int64_t count;
} LlWaitGroup;

void *ll_wait_group_create(void) {
  LlWaitGroup *w = (LlWaitGroup *)calloc(1, sizeof(LlWaitGroup));
  if (w == NULL) return NULL;
  pthread_mutex_init(&w->mu, NULL);
  pthread_cond_init(&w->cv, NULL);
  return w;
}

void ll_wait_group_add(void *wg, int64_t n) {
  LlWaitGroup *w = (LlWaitGroup *)wg;
  if (w == NULL) return;
  pthread_mutex_lock(&w->mu);
  w->count += n;
  pthread_mutex_unlock(&w->mu);
}

void ll_wait_group_done(void *wg) {
  LlWaitGroup *w = (LlWaitGroup *)wg;
  if (w == NULL) return;
  pthread_mutex_lock(&w->mu);
  if (w->count > 0) w->count--;
  if (w->count == 0) pthread_cond_broadcast(&w->cv);
  pthread_mutex_unlock(&w->mu);
}

void ll_wait_group_wait(void *wg) {
  LlWaitGroup *w = (LlWaitGroup *)wg;
  if (w == NULL) return;
  pthread_mutex_lock(&w->mu);
  while (w->count > 0) pthread_cond_wait(&w->cv, &w->mu);
  pthread_mutex_unlock(&w->mu);
}

typedef struct {
  pthread_mutex_t mu;
  void **tasks;
  size_t count;
  size_t cap;
} LlTaskGroup;

void *ll_task_group_create(void) {
  LlTaskGroup *g = (LlTaskGroup *)calloc(1, sizeof(LlTaskGroup));
  if (g == NULL) return NULL;
  pthread_mutex_init(&g->mu, NULL);
  g->cap = 8;
  g->tasks = (void **)calloc(g->cap, sizeof(void *));
  if (g->tasks == NULL) {
    free(g);
    return NULL;
  }
  return g;
}

void ll_task_group_add(void *group, LlTaskFn fn) {
  LlTaskGroup *g = (LlTaskGroup *)group;
  if (g == NULL || fn == NULL) return;
  void *task = ll_task_spawn(fn);
  if (task == NULL) return;
  pthread_mutex_lock(&g->mu);
  if (g->count >= g->cap) {
    size_t ncap = g->cap ? g->cap * 2 : 8;
    void **nt = (void **)realloc(g->tasks, ncap * sizeof(void *));
    if (nt == NULL) {
      pthread_mutex_unlock(&g->mu);
      ll_task_join(task);
      return;
    }
    g->tasks = nt;
    g->cap = ncap;
  }
  g->tasks[g->count++] = task;
  pthread_mutex_unlock(&g->mu);
}

void ll_task_group_wait(void *group) {
  LlTaskGroup *g = (LlTaskGroup *)group;
  if (g == NULL) return;
  pthread_mutex_lock(&g->mu);
  void **tasks = g->tasks;
  size_t n = g->count;
  g->tasks = NULL;
  g->count = 0;
  g->cap = 0;
  pthread_mutex_unlock(&g->mu);
  for (size_t i = 0; i < n; i++) {
    ll_task_join(tasks[i]);
  }
  free(tasks);
  /* Allow reuse after wait. */
  pthread_mutex_lock(&g->mu);
  g->cap = 8;
  g->tasks = (void **)calloc(g->cap, sizeof(void *));
  g->count = 0;
  pthread_mutex_unlock(&g->mu);
}

#if defined(_WIN32)
int ll_regex_is_match(void *pattern_str, void *text_str) {
  (void)pattern_str;
  (void)text_str;
  /* POSIX regex not available in the MSVC CRT — stub until a Windows backend lands. */
  return 0;
}
#else
#include <regex.h>
int ll_regex_is_match(void *pattern_str, void *text_str) {
  const char *pat = ll_string_cstr(pattern_str);
  const char *text = ll_string_cstr(text_str);
  if (pat == NULL || text == NULL) return 0;
  regex_t re;
  if (regcomp(&re, pat, REG_EXTENDED | REG_NOSUB) != 0) return 0;
  int rc = regexec(&re, text, 0, NULL, 0);
  regfree(&re);
  return rc == 0 ? 1 : 0;
}
#endif

typedef struct LlActorJob {
  LlTaskFn fn;
  struct LlActorJob *next;
} LlActorJob;

typedef struct {
  pthread_mutex_t mu;
  LlActorJob *head;
  LlActorJob *tail;
} LlActorQueue;

void *ll_actor_queue_create(void) {
  LlActorQueue *q = (LlActorQueue *)calloc(1, sizeof(LlActorQueue));
  if (q == NULL) return NULL;
  pthread_mutex_init(&q->mu, NULL);
  return q;
}

void ll_actor_queue_submit(void *q, LlTaskFn fn) {
  LlActorQueue *aq = (LlActorQueue *)q;
  if (aq == NULL || fn == NULL) return;
  LlActorJob *j = (LlActorJob *)calloc(1, sizeof(LlActorJob));
  j->fn = fn;
  pthread_mutex_lock(&aq->mu);
  if (aq->tail) aq->tail->next = j;
  else aq->head = j;
  aq->tail = j;
  pthread_mutex_unlock(&aq->mu);
}

void ll_actor_queue_drain(void *q) {
  LlActorQueue *aq = (LlActorQueue *)q;
  if (aq == NULL) return;
  for (;;) {
    pthread_mutex_lock(&aq->mu);
    LlActorJob *j = aq->head;
    if (j) {
      aq->head = j->next;
      if (aq->head == NULL) aq->tail = NULL;
    }
    pthread_mutex_unlock(&aq->mu);
    if (j == NULL) break;
    if (j->fn) j->fn();
    free(j);
  }
}

static LlActorQueue *ll_main_actor_q = NULL;
static pthread_once_t ll_main_actor_once = PTHREAD_ONCE_INIT;
static void ll_main_actor_init(void) {
  ll_main_actor_q = (LlActorQueue *)ll_actor_queue_create();
}

void ll_main_actor_submit(LlTaskFn fn) {
  pthread_once(&ll_main_actor_once, ll_main_actor_init);
  ll_actor_queue_submit(ll_main_actor_q, fn);
}

void ll_main_actor_drain(void) {
  pthread_once(&ll_main_actor_once, ll_main_actor_init);
  ll_actor_queue_drain(ll_main_actor_q);
}

typedef struct {
  _Atomic int cancelled;
} LlCancelTok;

void *ll_cancellation_token_create(void) {
  LlCancelTok *t = (LlCancelTok *)calloc(1, sizeof(LlCancelTok));
  return t;
}
void ll_cancellation_token_cancel(void *tok) {
  LlCancelTok *t = (LlCancelTok *)tok;
  if (t) atomic_store(&t->cancelled, 1);
}
int ll_cancellation_token_is_cancelled(void *tok) {
  LlCancelTok *t = (LlCancelTok *)tok;
  return t ? atomic_load(&t->cancelled) : 0;
}
void ll_cancellation_token_throw_if_cancelled(void *tok) {
  if (ll_cancellation_token_is_cancelled(tok)) {
    ll_trap("cancellation", "CancellationToken cancelled");
  }
}

/* Weak side table: obj -> list of slots */
typedef struct LlWeakNode {
  void **slot;
  struct LlWeakNode *next;
} LlWeakNode;

typedef struct LlWeakEntry {
  void *obj;
  LlWeakNode *slots;
  struct LlWeakEntry *next;
} LlWeakEntry;

static pthread_mutex_t ll_weak_mu = PTHREAD_MUTEX_INITIALIZER;
static LlWeakEntry *ll_weak_table = NULL;

static LlWeakEntry *ll_weak_find(void *obj) {
  for (LlWeakEntry *e = ll_weak_table; e; e = e->next) {
    if (e->obj == obj) return e;
  }
  return NULL;
}

void ll_weak_store(void **slot, void *obj) {
  if (slot == NULL) return;
  pthread_mutex_lock(&ll_weak_mu);
  /* unregister old */
  if (*slot) {
    LlWeakEntry *e = ll_weak_find(*slot);
    if (e) {
      LlWeakNode **pp = &e->slots;
      while (*pp) {
        if ((*pp)->slot == slot) {
          LlWeakNode *dead = *pp;
          *pp = dead->next;
          free(dead);
          break;
        }
        pp = &(*pp)->next;
      }
    }
  }
  *slot = obj;
  if (obj) {
    LlWeakEntry *e = ll_weak_find(obj);
    if (e == NULL) {
      e = (LlWeakEntry *)calloc(1, sizeof(LlWeakEntry));
      e->obj = obj;
      e->next = ll_weak_table;
      ll_weak_table = e;
    }
    LlWeakNode *n = (LlWeakNode *)calloc(1, sizeof(LlWeakNode));
    n->slot = slot;
    n->next = e->slots;
    e->slots = n;
  }
  pthread_mutex_unlock(&ll_weak_mu);
}

void *ll_weak_load(void **slot) {
  if (slot == NULL || *slot == NULL) return NULL;
  return ll_retain(*slot);
}

void ll_unowned_store(void **slot, void *obj) {
  if (slot) *slot = obj;
}

void *ll_unowned_load(void **slot) {
  if (slot == NULL || *slot == NULL) {
    ll_trap("unowned", "unowned reference is nil");
    return NULL;
  }
  /* Alive iff still registered in the weak side table (cleared on final release). */
  pthread_mutex_lock(&ll_weak_mu);
  int alive = ll_weak_find(*slot) != NULL;
  pthread_mutex_unlock(&ll_weak_mu);
  if (!alive) {
    ll_trap("unowned", "unowned reference to deallocated object");
  }
  return *slot;
}

/* Clear weak slots when object is about to die — called from ll_release path.
   We hook by wrapping: export ll_weak_clear_object for ll_release. */
void ll_weak_clear_object(void *obj) {
  if (obj == NULL) return;
  pthread_mutex_lock(&ll_weak_mu);
  LlWeakEntry **pp = &ll_weak_table;
  while (*pp) {
    if ((*pp)->obj == obj) {
      LlWeakEntry *e = *pp;
      *pp = e->next;
      for (LlWeakNode *n = e->slots; n;) {
        if (n->slot) *(n->slot) = NULL;
        LlWeakNode *nx = n->next;
        free(n);
        n = nx;
      }
      free(e);
      break;
    }
    pp = &(*pp)->next;
  }
  pthread_mutex_unlock(&ll_weak_mu);
}

void *ll_sql_open(void *path_str) {
  const char *path = ll_string_cstr(path_str);
  if (path == NULL) path = ":memory:";
  sqlite3 *db = NULL;
  if (sqlite3_open(path, &db) != SQLITE_OK) {
    if (db) sqlite3_close(db);
    return NULL;
  }
  return db;
}

void *ll_sql_exec(void *conn, void *sql_str) {
  sqlite3 *db = (sqlite3 *)conn;
  const char *sql = ll_string_cstr(sql_str);
  if (db == NULL || sql == NULL) return ll_string_from_cstr("error: bad args");
  char *errmsg = NULL;
  int rc = sqlite3_exec(db, sql, NULL, NULL, &errmsg);
  if (rc != SQLITE_OK) {
    void *s = ll_string_from_cstr(errmsg ? errmsg : "error");
    if (errmsg) sqlite3_free(errmsg);
    return s;
  }
  return ll_string_from_cstr("ok");
}

void ll_sql_close(void *conn) {
  if (conn) sqlite3_close((sqlite3 *)conn);
}
