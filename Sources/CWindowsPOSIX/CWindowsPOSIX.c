#if defined(_WIN32)

#include "CWindowsPOSIX.h"

#include <windows.h>

int flock(int fd, int operation) {
    HANDLE handle = (HANDLE)_get_osfhandle(fd);
    if (handle == INVALID_HANDLE_VALUE) {
        errno = EBADF;
        return -1;
    }

    OVERLAPPED overlapped;
    memset(&overlapped, 0, sizeof(overlapped));
    if (operation & LOCK_UN) {
        if (!UnlockFileEx(handle, 0, MAXDWORD, MAXDWORD, &overlapped)) {
            errno = EACCES;
            return -1;
        }
        return 0;
    }

    DWORD flags = 0;
    if (operation & LOCK_EX) {
        flags |= LOCKFILE_EXCLUSIVE_LOCK;
    }
    if (operation & LOCK_NB) {
        flags |= LOCKFILE_FAIL_IMMEDIATELY;
    }
    if (!LockFileEx(handle, flags, 0, MAXDWORD, MAXDWORD, &overlapped)) {
        DWORD error = GetLastError();
        errno = (error == ERROR_LOCK_VIOLATION || error == ERROR_IO_PENDING) ? EWOULDBLOCK : EACCES;
        return -1;
    }
    return 0;
}

int fcntl(int fd, int cmd, ...) {
    (void)fd;
    (void)cmd;
    return 0;
}

int pipe(int fds[2]) {
    return _pipe(fds, 4096, _O_BINARY | _O_NOINHERIT);
}

int open(const char *path, int oflag, ...) {
    int mode = 0;
    if (oflag & O_CREAT) {
        va_list args;
        va_start(args, oflag);
        mode = va_arg(args, int);
        va_end(args);
    }
    return _open(path, oflag | _O_BINARY, mode);
}

int close(int fd) {
    return _close(fd);
}

ssize_t read(int fd, void *buffer, size_t count) {
    unsigned int chunk = count > 0xffffffffu ? 0xffffffffu : (unsigned int)count;
    return (ssize_t)_read(fd, buffer, chunk);
}

ssize_t write(int fd, const void *buffer, size_t count) {
    unsigned int chunk = count > 0xffffffffu ? 0xffffffffu : (unsigned int)count;
    return (ssize_t)_write(fd, buffer, chunk);
}

int isatty(int fd) {
    return _isatty(fd);
}

int fstat(int fd, struct stat *buf) {
    return _fstat(fd, buf);
}

int access(const char *path, int mode) {
    return _access(path, mode);
}

int unlink(const char *path) {
    return _unlink(path);
}

#ifdef rename
#undef rename
#endif
int rename(const char *source, const char *destination) {
    if (MoveFileExA(source, destination, MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED)) {
        return 0;
    }
    errno = EACCES;
    return -1;
}

int dup(int fd) {
    return _dup(fd);
}

int dup2(int fd, int fd2) {
    return _dup2(fd, fd2);
}

int ioctl(int fd, unsigned long request, ...) {
    (void)fd;
    (void)request;
    errno = ENOSYS;
    return -1;
}

#ifdef chmod
#undef chmod
#endif
int chmod(const char *path, mode_t mode) {
    return _chmod(path, mode);
}

int lstat(const char *path, struct stat *buf) {
    return stat(path, buf);
}

ssize_t pread(int fd, void *buffer, size_t count, off_t offset) {
    __int64 current = _lseeki64(fd, 0, SEEK_CUR);
    if (current < 0 || _lseeki64(fd, offset, SEEK_SET) < 0) {
        return -1;
    }
    ssize_t result = read(fd, buffer, count);
    if (_lseeki64(fd, current, SEEK_SET) < 0) {
        return -1;
    }
    return result;
}

ssize_t pwrite(int fd, const void *buffer, size_t count, off_t offset) {
    __int64 current = _lseeki64(fd, 0, SEEK_CUR);
    if (current < 0 || _lseeki64(fd, offset, SEEK_SET) < 0) {
        return -1;
    }
    ssize_t result = write(fd, buffer, count);
    if (_lseeki64(fd, current, SEEK_SET) < 0) {
        return -1;
    }
    return result;
}

int ftruncate(int fd, off_t length) {
    return _chsize_s(fd, length) == 0 ? 0 : -1;
}

struct DIR {
    HANDLE handle;
    WIN32_FIND_DATAA find;
    int consumed;
    struct dirent entry;
};

DIR *opendir(const char *path) {
    char pattern[MAX_PATH];
    if (path == NULL) {
        errno = EINVAL;
        return NULL;
    }
    if (snprintf(pattern, MAX_PATH, "%s\\*", path) >= MAX_PATH) {
        errno = EINVAL;
        return NULL;
    }
    DIR *directory = (DIR *)calloc(1, sizeof(DIR));
    if (directory == NULL) {
        return NULL;
    }
    directory->handle = FindFirstFileA(pattern, &directory->find);
    if (directory->handle == INVALID_HANDLE_VALUE) {
        free(directory);
        errno = ENOENT;
        return NULL;
    }
    directory->consumed = 0;
    return directory;
}

struct dirent *readdir(DIR *directory) {
    if (directory == NULL || directory->handle == INVALID_HANDLE_VALUE) {
        return NULL;
    }
    if (directory->consumed) {
        if (!FindNextFileA(directory->handle, &directory->find)) {
            return NULL;
        }
    }
    directory->consumed = 1;
    memset(&directory->entry, 0, sizeof(directory->entry));
    strncpy(directory->entry.d_name, directory->find.cFileName, sizeof(directory->entry.d_name) - 1);
    return &directory->entry;
}

int closedir(DIR *directory) {
    if (directory == NULL) {
        return -1;
    }
    if (directory->handle != INVALID_HANDLE_VALUE) {
        FindClose(directory->handle);
    }
    free(directory);
    return 0;
}

int kill(pid_t pid, int sig) {
    if (pid <= 0) {
        errno = ENOSYS;
        return -1;
    }
    if (sig == 0) {
        HANDLE probe = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, (DWORD)pid);
        if (probe == NULL) {
            errno = EPERM;
            return -1;
        }
        CloseHandle(probe);
        return 0;
    }
    HANDLE handle = OpenProcess(PROCESS_TERMINATE, FALSE, (DWORD)pid);
    if (handle == NULL) {
        errno = EPERM;
        return -1;
    }
    BOOL ok = TerminateProcess(handle, 1);
    CloseHandle(handle);
    if (!ok) {
        errno = EPERM;
        return -1;
    }
    return 0;
}

int setpgid(pid_t pid, pid_t pgid) {
    (void)pid;
    (void)pgid;
    errno = ENOSYS;
    return -1;
}

pid_t getpgid(pid_t pid) {
    return pid;
}

pid_t getpgrp(void) {
    return (pid_t)_getpid();
}

#ifdef getpid
#undef getpid
#endif
pid_t getpid(void) {
    return (pid_t)_getpid();
}

uid_t getuid(void) {
    return 0;
}

uid_t geteuid(void) {
    return 0;
}

int fchmod(int fd, mode_t mode) {
    (void)fd;
    (void)mode;
    return 0;
}

#ifdef mkdir
#undef mkdir
#endif
int mkdir(const char *path, mode_t mode) {
    (void)mode;
    return _mkdir(path);
}

char *realpath(const char *path, char *resolved) {
    char buffer[PATH_MAX];
    DWORD length = GetFullPathNameA(path, PATH_MAX, buffer, NULL);
    if (length == 0 || length >= PATH_MAX) {
        errno = EINVAL;
        return NULL;
    }
    if (resolved == NULL) {
        return _strdup(buffer);
    }
    memcpy(resolved, buffer, length + 1);
    return resolved;
}

int fstatat(int dirfd, const char *pathname, struct stat *buf, int flags) {
    (void)dirfd;
    (void)flags;
    return stat(pathname, buf);
}

int waitid(idtype_t idtype, id_t id, siginfo_t *infop, int options) {
    (void)idtype;
    (void)options;
    HANDLE handle = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, (DWORD)id);
    if (handle == NULL) {
        errno = EPERM;
        return -1;
    }
    DWORD wait = WaitForSingleObject(handle, INFINITE);
    DWORD exitCode = 1;
    GetExitCodeProcess(handle, &exitCode);
    CloseHandle(handle);
    if (wait != WAIT_OBJECT_0) {
        errno = EPERM;
        return -1;
    }
    if (infop != NULL) {
        memset(infop, 0, sizeof(*infop));
        infop->si_pid = (pid_t)id;
        infop->si_status = (int)exitCode;
    }
    return 0;
}

pid_t waitpid(pid_t pid, int *status, int options) {
    (void)options;
    HANDLE handle = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, (DWORD)pid);
    if (handle == NULL) {
        errno = EPERM;
        return -1;
    }
    DWORD wait = WaitForSingleObject(handle, INFINITE);
    DWORD exitCode = 1;
    GetExitCodeProcess(handle, &exitCode);
    CloseHandle(handle);
    if (wait != WAIT_OBJECT_0) {
        errno = EPERM;
        return -1;
    }
    if (status != NULL) {
        *status = ((int)exitCode) << 8;
    }
    return pid;
}

unsigned int usleep(useconds_t usec) {
    DWORD millis = usec == 0 ? 0 : (DWORD)((usec + 999) / 1000);
    Sleep(millis);
    return 0;
}

int openpty(int *amaster, int *aslave, char *name, const void *termp, const winsize *winp) {
    (void)amaster;
    (void)aslave;
    (void)name;
    (void)termp;
    (void)winp;
    errno = ENOSYS;
    return -1;
}

int posix_spawn(
    pid_t *pid,
    const char *path,
    const posix_spawn_file_actions_t *file_actions,
    const posix_spawnattr_t *attrp,
    char *const argv[],
    char *const envp[])
{
    (void)pid;
    (void)path;
    (void)file_actions;
    (void)attrp;
    (void)argv;
    (void)envp;
    errno = ENOSYS;
    return ENOSYS;
}

int posix_spawn_file_actions_init(posix_spawn_file_actions_t *file_actions) {
    if (file_actions == NULL) {
        return EINVAL;
    }
    file_actions->reserved = 0;
    return 0;
}

int posix_spawn_file_actions_destroy(posix_spawn_file_actions_t *file_actions) {
    (void)file_actions;
    return 0;
}

int posix_spawn_file_actions_addopen(
    posix_spawn_file_actions_t *file_actions,
    int fildes,
    const char *path,
    int oflag,
    mode_t mode)
{
    (void)file_actions;
    (void)fildes;
    (void)path;
    (void)oflag;
    (void)mode;
    return 0;
}

int posix_spawn_file_actions_adddup2(posix_spawn_file_actions_t *file_actions, int fildes, int newfildes) {
    (void)file_actions;
    (void)fildes;
    (void)newfildes;
    return 0;
}

int posix_spawn_file_actions_addclose(posix_spawn_file_actions_t *file_actions, int fildes) {
    (void)file_actions;
    (void)fildes;
    return 0;
}

int posix_spawn_file_actions_addchdir_np(posix_spawn_file_actions_t *file_actions, const char *path) {
    (void)file_actions;
    (void)path;
    return 0;
}

int posix_spawnattr_init(posix_spawnattr_t *attr) {
    if (attr == NULL) {
        return EINVAL;
    }
    attr->reserved = 0;
    return 0;
}

int posix_spawnattr_destroy(posix_spawnattr_t *attr) {
    (void)attr;
    return 0;
}

int posix_spawnattr_setsigmask(posix_spawnattr_t *attr, const sigset_t *sigmask) {
    (void)attr;
    (void)sigmask;
    return 0;
}

int posix_spawnattr_setsigdefault(posix_spawnattr_t *attr, const sigset_t *sigdefault) {
    (void)attr;
    (void)sigdefault;
    return 0;
}

int posix_spawnattr_setflags(posix_spawnattr_t *attr, short flags) {
    (void)attr;
    (void)flags;
    return 0;
}

int posix_spawnattr_setpgroup(posix_spawnattr_t *attr, pid_t pgroup) {
    (void)attr;
    (void)pgroup;
    return 0;
}

int sigemptyset(sigset_t *set) {
    if (set == NULL) {
        return -1;
    }
    *set = 0;
    return 0;
}

int sigaddset(sigset_t *set, int signo) {
    if (set == NULL) {
        return -1;
    }
    (void)signo;
    return 0;
}

#else
typedef int codexbar_cwindows_posix_unused;
#endif
