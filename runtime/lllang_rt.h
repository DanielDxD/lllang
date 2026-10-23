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

/* Runtime type check: compare object's type descriptor name to `type_name` (String). */
int ll_isa(void *obj, void *type_name_str);
const char *ll_type_name(void *obj);

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
/* `arr` is an in/out handle so COW can replace the caller's pointer. */
void ll_array_append(void **arr, const void *elem);
size_t ll_array_count(void *arr);
void *ll_array_get_ptr(void *arr, size_t index);

/* Convenience for Int arrays (Phase 5). */
void *ll_array_new_i64(void);
void ll_array_append_i64(void **arr, int64_t v);
int64_t ll_array_get_i64(void *arr, size_t index);
void ll_array_set_i64(void **arr, size_t index, int64_t v);
void *ll_array_sorted_i64(void *arr);

/* --- IO / traps / Int --- */

void ll_print(void *str);
void ll_eprint(void *str);
void *ll_read_line(void); /* String? as null = EOF */
void ll_trap(const char *kind, const char *msg);
void ll_die(void *msg_str);
void ll_fatal_error(void *msg_str);
void ll_assert(int cond, void *msg_str);

/* Env / FS / Time / Process (Phase 6). */
void ll_rt_set_args(int argc, char **argv);
void *ll_args_get(size_t index); /* String; null if OOB */
int64_t ll_args_count(void);
void *ll_env_get(void *key_str); /* String?; null if missing */
void ll_env_load_dotenv(void *path_str);
void *ll_env_get_string_or(void *key_str, void *default_str);
int64_t ll_env_get_i64(void *key_str, int *ok_out);
int64_t ll_env_get_i64_or(void *key_str, int64_t default_val);
int ll_env_get_bool(void *key_str, int *ok_out);
int ll_env_get_bool_or(void *key_str, int default_val);
double ll_env_get_f64(void *key_str, int *ok_out);
double ll_env_get_f64_or(void *key_str, double default_val);
int64_t ll_env_get_decimal(void *key_str, int *ok_out); /* scaled cents */
int64_t ll_env_get_decimal_or(void *key_str, int64_t default_val);
int ll_file_exists(void *path_str);
void *ll_file_read_text(void *path_str);
int ll_file_write_text(void *path_str, void *contents_str);
int64_t ll_instant_now_ms(void);
void ll_sleep_ms(int64_t ms);
void ll_process_exit(int64_t code);
void *ll_process_start(void *cmd_str); /* opaque handle; MVP runs immediately */
int64_t ll_process_wait(void *handle); /* exit code; frees handle */

/* String helpers */
int ll_string_contains(void *hay, void *needle);
void *ll_string_trim(void *str);
void *ll_string_substring(void *str, int64_t start, int64_t end);
void *ll_string_split(void *str, void *sep); /* Array of String */
void *ll_string_upper(void *str);
void *ll_string_lower(void *str);
void *ll_string_replace(void *str, void *old_str, void *new_str);
void *ll_string_join(void *arr, void *sep); /* Array of String → String */
int64_t ll_int_from_string(void *str, int *ok_out);
double ll_double_sqrt(double x);

/* Identity (non-crypto-grade) */
void *ll_identity_uuid(void);
void *ll_identity_parse(void *s);
void *ll_identity_short_id(void);
void *ll_identity_ulid(void);
void *ll_identity_nanoid(void);

/* Math */
int64_t ll_math_abs_i64(int64_t x);
int64_t ll_math_max_i64(int64_t a, int64_t b);
int64_t ll_math_min_i64(int64_t a, int64_t b);
int64_t ll_math_clamp_i64(int64_t v, int64_t lo, int64_t hi);
double ll_math_sqrt(double x);
double ll_math_pow(double x, double y);
double ll_math_sin(double x);
double ll_math_log10(double x);
double ll_math_pi(void);

/* Decimal as scaled int64 (cents: value * 100) */
int64_t ll_decimal_from_string(void *str, int *ok_out);
int64_t ll_decimal_add(int64_t a, int64_t b);
int64_t ll_decimal_sub(int64_t a, int64_t b);
int64_t ll_decimal_mul(int64_t a, int64_t b);

/* Crypto helpers */
void *ll_base64_encode(void *data_str);
void *ll_hex_encode(void *data_str);
void *ll_secure_random_bytes(int64_t count); /* opaque String of raw bytes */

/* Pointer arrays (String elements, etc.) */
void *ll_array_new_ptr(void);
void ll_array_append_ptr(void **arr, void *elem);
void *ll_array_get_ref(void *arr, size_t index);

/* Dictionary / Set (string keys, Phase 6 MVP) */
void *ll_dict_new(void);
void ll_dict_set(void *dict, void *key_str, void *val_str);
void *ll_dict_get(void *dict, void *key_str); /* String?; null if missing */
int ll_dict_contains(void *dict, void *key_str);
int64_t ll_dict_count(void *dict);
void *ll_dict_keys(void *dict); /* Array of String */
void *ll_set_new(void);
void ll_set_insert(void *set, void *key_str);
int ll_set_contains(void *set, void *key_str);
int64_t ll_set_count(void *set);
void *ll_set_union(void *a, void *b);
void *ll_set_intersection(void *a, void *b);
void *ll_set_difference(void *a, void *b);

/* --- JSON / TOON MVP (schema: "a:s,b:i,c:b,d:{x:s},e:[i],...") --- */
int ll_json_decode_fields(void *json_str, const char *schema, void *out_struct);
void *ll_json_encode_fields(const char *schema, const void *in_struct);
int ll_toon_decode_fields(void *toon_str, const char *schema, void *out_struct);
void *ll_toon_encode_fields(const char *schema, const void *in_struct);

/* --- HTTP / LLM MVP --- */
void *ll_http_get(void *url_str, int *ok_out);
void *ll_http_post(void *url_str, void *body_str, int *ok_out);
void *ll_llm_run_agent(void *model_str, void *tools_str, void *user_str);
void *ll_llm_merge_context(void *package_name);
void ll_noop(void);

/* --- Phase 8 Std.* / FFI --- */
void ll_log_debug(void *msg);
void ll_log_debug_fields(void *msg, void *fields_str);
void *ll_arena_create(int64_t bytes);
void ll_run_blocking_noop(void);
int64_t ll_with_timeout_ms(int64_t ms);
void ll_http_server_listen(int64_t port, void *body_str);
/* Durable accept loop; handler(path_string) -> body_string (ARC String). */
typedef void *(*LlHttpHandler)(void *path_str);
void ll_http_server_serve(int64_t port, LlHttpHandler handler);
void ll_log_info(void *msg);
void ll_log_warn(void *msg);
void ll_log_error(void *msg);
void ll_cli_usage(void *text);
int ll_cli_has_flag(void *flag_str);
void *ll_tcp_connect(void *host_str, int64_t port); /* TcpStream handle or null */
void *ll_tcp_read(void *stream);                    /* String */
void ll_tcp_write(void *stream, void *data_str);
void ll_tcp_close(void *stream);
void *ll_mutex_create(void);
void ll_mutex_lock(void *mutex);
void ll_mutex_unlock(void *mutex);
void *ll_crypto_sha256(void *data_str); /* hex String */
void *ll_crypto_hmac_sha256(void *key_str, void *data_str); /* hex String */
int64_t ll_c_strlen(void *str);         /* FFI smoke: strlen on string bytes */

/* --- Testing --- */
void ll_test_expect(bool ok, void *msg);
void ll_test_expect_empty(void *str, void *msg);
void ll_test_expect_not_empty(void *str, void *msg);
void ll_test_contains(void *hay, void *needle, void *msg);
void ll_test_approx(double a, double b, void *msg);
int64_t ll_test_job_id(void);

/* --- Phase 10 concurrency (thread pool + joinable tasks) --- */
typedef void (*LlTaskFn)(void);
typedef void *(*LlTaskRetFn)(void);
void *ll_task_spawn(LlTaskFn fn); /* Task handle; joinable */
void *ll_task_spawn_ret(LlTaskRetFn fn); /* Task whose fn returns a pointer result */
void ll_task_join(void *task);
void *ll_task_join_ret(void *task); /* join and return pointer result (or NULL) */
void ll_task_detach(void *task); /* fire-and-forget */
int ll_task_is_cancelled(void *task);
void ll_task_cancel(void *task);

void *ll_channel_create(int64_t capacity); /* i64 elements; 0 = unbuffered handoff */
void ll_channel_send_i64(void *ch, int64_t v);
int64_t ll_channel_recv_i64(void *ch);
void ll_channel_close(void *ch);

void *ll_wait_group_create(void);
void ll_wait_group_add(void *wg, int64_t n);
void ll_wait_group_done(void *wg);
void ll_wait_group_wait(void *wg);

/* TaskGroup — spawn zero-arg workers and join them together */
void *ll_task_group_create(void);
void ll_task_group_add(void *group, LlTaskFn fn); /* spawn fn into group */
void ll_task_group_wait(void *group);             /* join all members */

/* Regex MVP — POSIX Extended on Unix; stub on Windows */
int ll_regex_is_match(void *pattern_str, void *text_str);

/* Actor / main queue (serial) */
void *ll_actor_queue_create(void);
void ll_actor_queue_submit(void *q, LlTaskFn fn);
void ll_actor_queue_drain(void *q); /* run until empty (sync helper) */
void ll_main_actor_submit(LlTaskFn fn);
void ll_main_actor_drain(void);

/* Cancellation token */
void *ll_cancellation_token_create(void);
void ll_cancellation_token_cancel(void *tok);
int ll_cancellation_token_is_cancelled(void *tok);
void ll_cancellation_token_throw_if_cancelled(void *tok); /* traps if cancelled */

/* --- Phase 10 weak / unowned --- */
void ll_weak_store(void **slot, void *obj);   /* slot holds weak ref to obj (or NULL) */
void *ll_weak_load(void **slot);              /* strong retain or NULL */
void ll_unowned_store(void **slot, void *obj);
void *ll_unowned_load(void **slot);           /* traps if dangling */

/* --- Phase 10 SQL (sqlite) --- */
void *ll_sql_open(void *path_str); /* connection handle; null on error */
void *ll_sql_exec(void *conn, void *sql_str); /* result String (ok/error message) */
void ll_sql_close(void *conn);

/* User program entry (defined by compiled LLLang). */
void ll_main(void);

#endif /* LLLANG_RT_H */
