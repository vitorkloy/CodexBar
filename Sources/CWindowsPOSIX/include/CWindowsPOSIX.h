#ifndef CODEXBAR_CWINDOWS_POSIX_H
#define CODEXBAR_CWINDOWS_POSIX_H

#if defined(_WIN32)

#include <errno.h>
#include <fcntl.h>
#include <io.h>
#include <process.h>
#include <signal.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>

#ifndef STDIN_FILENO
#define STDIN_FILENO 0
#endif
#ifndef STDOUT_FILENO
#define STDOUT_FILENO 1
#endif
#ifndef STDERR_FILENO
#define STDERR_FILENO 2
#endif

#ifndef O_RDONLY
#define O_RDONLY _O_RDONLY
#endif
#ifndef O_WRONLY
#define O_WRONLY _O_WRONLY
#endif
#ifndef O_RDWR
#define O_RDWR _O_RDWR
#endif
#ifndef O_CREAT
#define O_CREAT _O_CREAT
#endif
#ifndef O_EXCL
#define O_EXCL _O_EXCL
#endif
#ifndef O_TRUNC
#define O_TRUNC _O_TRUNC
#endif
#ifndef O_APPEND
#define O_APPEND _O_APPEND
#endif
#ifndef O_BINARY
#define O_BINARY _O_BINARY
#endif
#ifndef O_CLOEXEC
#define O_CLOEXEC _O_NOINHERIT
#endif
#ifndef O_NOFOLLOW
#define O_NOFOLLOW 0
#endif
#ifndef O_NONBLOCK
#define O_NONBLOCK 0
#endif
#ifndef O_NOCTTY
#define O_NOCTTY 0
#endif

#ifndef S_IFMT
#define S_IFMT _S_IFMT
#endif
#ifndef S_IFREG
#define S_IFREG _S_IFREG
#endif
#ifndef S_IFDIR
#define S_IFDIR _S_IFDIR
#endif
#ifndef S_IRUSR
#define S_IRUSR _S_IREAD
#endif
#ifndef S_IWUSR
#define S_IWUSR _S_IWRITE
#endif
#ifndef S_IXUSR
#define S_IXUSR _S_IEXEC
#endif
#ifndef S_IRWXU
#define S_IRWXU (S_IRUSR | S_IWUSR | S_IXUSR)
#endif

#ifndef LOCK_SH
#define LOCK_SH 1
#endif
#ifndef LOCK_EX
#define LOCK_EX 2
#endif
#ifndef LOCK_NB
#define LOCK_NB 4
#endif
#ifndef LOCK_UN
#define LOCK_UN 8
#endif

#ifndef F_GETFD
#define F_GETFD 1
#endif
#ifndef F_SETFD
#define F_SETFD 2
#endif
#ifndef F_GETFL
#define F_GETFL 3
#endif
#ifndef F_SETFL
#define F_SETFL 4
#endif
#ifndef FD_CLOEXEC
#define FD_CLOEXEC 1
#endif
#ifndef AT_FDCWD
#define AT_FDCWD (-100)
#endif
#ifndef PATH_MAX
#define PATH_MAX 4096
#endif
#ifndef EPERM
#define EPERM 1
#endif
#ifndef EINTR
#define EINTR 4
#endif
#ifndef EAGAIN
#define EAGAIN 11
#endif
#ifndef EACCES
#define EACCES 13
#endif
#ifndef EWOULDBLOCK
#define EWOULDBLOCK EAGAIN
#endif
#ifndef ENOSYS
#define ENOSYS 38
#endif
#ifndef EBADF
#define EBADF 9
#endif
#ifndef ENOTDIR
#define ENOTDIR 20
#endif
#ifndef EINVAL
#define EINVAL 22
#endif
#ifndef ENOENT
#define ENOENT 2
#endif

#ifndef SIGHUP
#define SIGHUP 1
#endif
#ifndef SIGINT
#define SIGINT 2
#endif
#ifndef SIGKILL
#define SIGKILL 9
#endif
#ifndef SIGTERM
#define SIGTERM 15
#endif
#ifndef SIGPIPE
#define SIGPIPE 13
#endif

#ifndef R_OK
#define R_OK 4
#endif
#ifndef W_OK
#define W_OK 2
#endif
#ifndef X_OK
#define X_OK 1
#endif
#ifndef F_OK
#define F_OK 0
#endif

#ifndef WEXITED
#define WEXITED 4
#endif
#ifndef WNOWAIT
#define WNOWAIT 0x01000000
#endif
#ifndef WNOHANG
#define WNOHANG 1
#endif
#ifndef WIFEXITED
#define WIFEXITED(status) (((status) & 0x7f) == 0)
#endif
#ifndef WEXITSTATUS
#define WEXITSTATUS(status) (((status) >> 8) & 0xff)
#endif
#ifndef WIFSIGNALED
#define WIFSIGNALED(status) (((status) & 0x7f) != 0 && ((status) & 0x7f) != 0x7f)
#endif
#ifndef WTERMSIG
#define WTERMSIG(status) ((status) & 0x7f)
#endif
#ifndef SIG_DFL
#define SIG_DFL ((void (*)(int))0)
#endif
#ifndef SIG_IGN
#define SIG_IGN ((void (*)(int))1)
#endif
#ifndef TIOCGWINSZ
#define TIOCGWINSZ 0
#endif

#ifdef __cplusplus
extern "C" {
#endif

#ifndef _UID_T_DEFINED
typedef unsigned int uid_t;
#define _UID_T_DEFINED
#endif
#ifndef _GID_T_DEFINED
typedef unsigned int gid_t;
#define _GID_T_DEFINED
#endif
#ifndef _PID_T_DEFINED
typedef int pid_t;
#define _PID_T_DEFINED
#endif
#ifndef _MODE_T_DEFINED
typedef unsigned short mode_t;
#define _MODE_T_DEFINED
#endif
#ifndef _ID_T_DEFINED
typedef int id_t;
#define _ID_T_DEFINED
#endif
#ifndef _USECONDS_T_DEFINED
typedef unsigned int useconds_t;
#define _USECONDS_T_DEFINED
#endif
#ifndef _SSIZE_T_DEFINED
typedef intptr_t ssize_t;
#define _SSIZE_T_DEFINED
#endif
#ifndef _OFF_T_DEFINED
typedef long long off_t;
#define _OFF_T_DEFINED
#endif

#ifndef DT_UNKNOWN
#define DT_UNKNOWN 0
#endif

struct dirent {
    unsigned char d_type;
    char d_name[260];
};

typedef struct DIR DIR;

typedef int idtype_t;
#ifndef P_PID
#define P_PID 1
#endif

typedef struct {
    int si_signo;
    int si_code;
    int si_errno;
    int si_status;
    pid_t si_pid;
} siginfo_t;

typedef struct {
    unsigned short ws_row;
    unsigned short ws_col;
    unsigned short ws_xpixel;
    unsigned short ws_ypixel;
} winsize;

typedef struct {
    int reserved;
} posix_spawn_file_actions_t;

typedef struct {
    int reserved;
} posix_spawnattr_t;

typedef unsigned long sigset_t;

#ifdef open
#undef open
#endif
#ifdef close
#undef close
#endif
#ifdef read
#undef read
#endif
#ifdef write
#undef write
#endif
#ifdef isatty
#undef isatty
#endif
#ifdef fstat
#undef fstat
#endif
#ifdef access
#undef access
#endif
#ifdef unlink
#undef unlink
#endif
#ifdef rename
#undef rename
#endif
#ifdef dup
#undef dup
#endif
#ifdef dup2
#undef dup2
#endif

int flock(int fd, int operation);
int fcntl(int fd, int cmd, ...);
int pipe(int fds[2]);
int open(const char *path, int oflag, ...);
int close(int fd);
ssize_t read(int fd, void *buffer, size_t count);
ssize_t write(int fd, const void *buffer, size_t count);
int isatty(int fd);
int fstat(int fd, struct stat *buf);
int access(const char *path, int mode);
int unlink(const char *path);
int rename(const char *source, const char *destination);
int dup(int fd);
int dup2(int fd, int fd2);
int ioctl(int fd, unsigned long request, ...);
int chmod(const char *path, mode_t mode);
int lstat(const char *path, struct stat *buf);
ssize_t pread(int fd, void *buffer, size_t count, off_t offset);
ssize_t pwrite(int fd, const void *buffer, size_t count, off_t offset);
int ftruncate(int fd, off_t length);
DIR *opendir(const char *path);
struct dirent *readdir(DIR *directory);
int closedir(DIR *directory);
int kill(pid_t pid, int sig);
int setpgid(pid_t pid, pid_t pgid);
pid_t getpgid(pid_t pid);
pid_t getpgrp(void);
pid_t getpid(void);
uid_t getuid(void);
uid_t geteuid(void);
int fchmod(int fd, mode_t mode);
int mkdir(const char *path, mode_t mode);
char *realpath(const char *path, char *resolved);
int fstatat(int dirfd, const char *pathname, struct stat *buf, int flags);
int waitid(idtype_t idtype, id_t id, siginfo_t *infop, int options);
pid_t waitpid(pid_t pid, int *status, int options);
unsigned int usleep(useconds_t usec);
int openpty(int *amaster, int *aslave, char *name, const void *termp, const winsize *winp);
int posix_spawn(
    pid_t *pid,
    const char *path,
    const posix_spawn_file_actions_t *file_actions,
    const posix_spawnattr_t *attrp,
    char *const argv[],
    char *const envp[]);
int posix_spawn_file_actions_init(posix_spawn_file_actions_t *file_actions);
int posix_spawn_file_actions_destroy(posix_spawn_file_actions_t *file_actions);
int posix_spawn_file_actions_addopen(
    posix_spawn_file_actions_t *file_actions,
    int fildes,
    const char *path,
    int oflag,
    mode_t mode);
int posix_spawn_file_actions_adddup2(posix_spawn_file_actions_t *file_actions, int fildes, int newfildes);
int posix_spawn_file_actions_addclose(posix_spawn_file_actions_t *file_actions, int fildes);
int posix_spawn_file_actions_addchdir_np(posix_spawn_file_actions_t *file_actions, const char *path);
int posix_spawnattr_init(posix_spawnattr_t *attr);
int posix_spawnattr_destroy(posix_spawnattr_t *attr);
int posix_spawnattr_setsigmask(posix_spawnattr_t *attr, const sigset_t *sigmask);
int posix_spawnattr_setsigdefault(posix_spawnattr_t *attr, const sigset_t *sigdefault);
int posix_spawnattr_setflags(posix_spawnattr_t *attr, short flags);
int posix_spawnattr_setpgroup(posix_spawnattr_t *attr, pid_t pgroup);
int sigemptyset(sigset_t *set);
int sigaddset(sigset_t *set, int signo);

#ifdef __cplusplus
}
#endif


#ifndef POSIX_SPAWN_SETPGROUP
#define POSIX_SPAWN_SETPGROUP 0x02
#endif
#ifndef POSIX_SPAWN_SETSIGMASK
#define POSIX_SPAWN_SETSIGMASK 0x08
#endif
#ifndef POSIX_SPAWN_SETSID
#define POSIX_SPAWN_SETSID 0x80
#endif

#else
typedef int codexbar_cwindows_posix_unused;
#endif /* _WIN32 */

#endif /* CODEXBAR_CWINDOWS_POSIX_H */
