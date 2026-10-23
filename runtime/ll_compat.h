/* Platform shims for the LLLang C runtime (pthread / sockets / strings). */
#ifndef LL_COMPAT_H
#define LL_COMPAT_H

#if defined(_WIN32)

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef _WINSOCKAPI_
/* winsock2 before windows.h to avoid winsock.h conflicts */
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#include <stdlib.h>

#ifndef strcasecmp
#define strcasecmp _stricmp
#endif
#ifndef strncasecmp
#define strncasecmp _strnicmp
#endif

#ifndef ssize_t
typedef intptr_t ssize_t;
#endif

/* --- pthread subset via Win32 --- */

typedef HANDLE pthread_t;
typedef SRWLOCK pthread_mutex_t;
typedef CONDITION_VARIABLE pthread_cond_t;
typedef INIT_ONCE pthread_once_t;

#define PTHREAD_MUTEX_INITIALIZER SRWLOCK_INIT
#define PTHREAD_ONCE_INIT INIT_ONCE_STATIC_INIT

static inline int pthread_mutex_init(pthread_mutex_t *m, void *attr) {
  (void)attr;
  InitializeSRWLock(m);
  return 0;
}
static inline int pthread_mutex_lock(pthread_mutex_t *m) {
  AcquireSRWLockExclusive(m);
  return 0;
}
static inline int pthread_mutex_unlock(pthread_mutex_t *m) {
  ReleaseSRWLockExclusive(m);
  return 0;
}
static inline int pthread_cond_init(pthread_cond_t *c, void *attr) {
  (void)attr;
  InitializeConditionVariable(c);
  return 0;
}
static inline int pthread_cond_wait(pthread_cond_t *c, pthread_mutex_t *m) {
  SleepConditionVariableSRW(c, m, INFINITE, 0);
  return 0;
}
static inline int pthread_cond_signal(pthread_cond_t *c) {
  WakeConditionVariable(c);
  return 0;
}
static inline int pthread_cond_broadcast(pthread_cond_t *c) {
  WakeAllConditionVariable(c);
  return 0;
}

typedef struct LlPthreadStart {
  void *(*start)(void *);
  void *arg;
} LlPthreadStart;

static DWORD WINAPI ll_pthread_trampoline(LPVOID p) {
  LlPthreadStart *s = (LlPthreadStart *)p;
  void *(*start)(void *) = s->start;
  void *arg = s->arg;
  free(s);
  start(arg);
  return 0;
}

static inline int pthread_create(pthread_t *t, void *attr, void *(*start)(void *), void *arg) {
  (void)attr;
  LlPthreadStart *s = (LlPthreadStart *)malloc(sizeof(LlPthreadStart));
  if (s == NULL) return -1;
  s->start = start;
  s->arg = arg;
  *t = CreateThread(NULL, 0, ll_pthread_trampoline, s, 0, NULL);
  if (*t == NULL) {
    free(s);
    return -1;
  }
  return 0;
}
static inline int pthread_join(pthread_t t, void **retval) {
  (void)retval;
  WaitForSingleObject(t, INFINITE);
  CloseHandle(t);
  return 0;
}
static inline int pthread_detach(pthread_t t) {
  CloseHandle(t);
  return 0;
}

static BOOL CALLBACK ll_pthread_once_cb(PINIT_ONCE once, PVOID param, PVOID *ctx) {
  (void)once;
  (void)ctx;
  ((void (*)(void))param)();
  return TRUE;
}
static inline int pthread_once(pthread_once_t *once, void (*init)(void)) {
  InitOnceExecuteOnce(once, ll_pthread_once_cb, (PVOID)init, NULL);
  return 0;
}

/* --- sockets --- */

static BOOL CALLBACK ll_winsock_once_cb(PINIT_ONCE once, PVOID param, PVOID *ctx) {
  (void)once;
  (void)param;
  (void)ctx;
  WSADATA wsa;
  WSAStartup(MAKEWORD(2, 2), &wsa);
  return TRUE;
}

static inline void ll_net_init(void) {
  static INIT_ONCE once = INIT_ONCE_STATIC_INIT;
  InitOnceExecuteOnce(&once, ll_winsock_once_cb, NULL, NULL);
}

#define LL_CLOSESOCKET(fd) closesocket(fd)
#define LL_READSOCK(fd, buf, n) recv((fd), (buf), (int)(n), 0)
#define LL_WRITESOCK(fd, buf, n) send((fd), (buf), (int)(n), 0)

#else /* ! _WIN32 */

#include <pthread.h>
#include <strings.h>
#include <sys/socket.h>
#include <netdb.h>
#include <unistd.h>
#include <arpa/inet.h>

static inline void ll_net_init(void) {}

#define LL_CLOSESOCKET(fd) close(fd)
#define LL_READSOCK(fd, buf, n) read((fd), (buf), (n))
#define LL_WRITESOCK(fd, buf, n) write((fd), (buf), (n))

#endif /* _WIN32 */

#endif /* LL_COMPAT_H */
