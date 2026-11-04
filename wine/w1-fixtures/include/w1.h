/* SPDX-License-Identifier: MIT OR Apache-2.0 */
#ifndef W1_H
#define W1_H

#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN64
#define W1_ARCH "x64"
#else
#define W1_ARCH "pe32"
#endif

#define CUDAAPI __stdcall

typedef int CUresult;
typedef int CUdevice;
typedef struct CUstream_st *CUstream;
typedef struct CUevent_st *CUevent;
typedef struct CUgraph_st *CUgraph;
typedef struct CUgraphExec_st *CUgraphExec;
typedef struct CUmemPoolHandle_st *CUmemoryPool;
/* cuda.h: CUdeviceptr is pointer-sized on the caller's architecture. */
typedef uintptr_t CUdeviceptr;
typedef unsigned long long CUmemGenericAllocationHandle;
typedef struct { unsigned char bytes[16]; } CUuuid;
typedef struct { char reserved[64]; } CUipcEventHandle;
typedef struct { char reserved[64]; } CUipcMemHandle;
typedef struct { int type; int id; } CUmemLocation;

#define CUDA_SUCCESS 0
#define CUDA_ERROR_NOT_SUPPORTED 801

static CRITICAL_SECTION w1_out_lock;
static int w1_failures;

static void w1_emit(const char *fmt, ...)
{
    va_list ap;
    EnterCriticalSection(&w1_out_lock);
    printf("{\"arch\":\"" W1_ARCH "\",");
    va_start(ap, fmt);
    vprintf(fmt, ap);
    va_end(ap);
    printf("}\n");
    fflush(stdout);
    LeaveCriticalSection(&w1_out_lock);
}

static void w1_check(const char *name, int ok, const char *fmt, ...)
{
    char detail[512];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(detail, sizeof detail, fmt, ap);
    va_end(ap);
    if (!ok)
        InterlockedIncrement((LONG *)&w1_failures);
    w1_emit("\"check\":\"%s\",\"pass\":%s,%s", name, ok ? "true" : "false", detail);
}

/* ---- deadline: a timeout is never success (exit 124) ---- */

static DWORD w1_deadline_ms;

static DWORD WINAPI w1_watchdog(void *unused)
{
    (void)unused;
    Sleep(w1_deadline_ms);
    w1_emit("\"event\":\"deadline\",\"deadline_ms\":%lu", (unsigned long)w1_deadline_ms);
    TerminateProcess(GetCurrentProcess(), 124);
    return 0;
}

static void w1_start(const char *fixture, DWORD deadline_ms)
{
    InitializeCriticalSection(&w1_out_lock);
    const char *env = getenv("W1_DEADLINE_MS");
    w1_deadline_ms = env ? (DWORD)strtoul(env, NULL, 10) : deadline_ms;
    HANDLE t = CreateThread(NULL, 0, w1_watchdog, NULL, 0, NULL);
    if (t)
        CloseHandle(t);
    w1_emit("\"event\":\"start\",\"fixture\":\"%s\",\"deadline_ms\":%lu,\"v_pid\":%lu", fixture,
            (unsigned long)w1_deadline_ms, (unsigned long)GetCurrentProcessId());
}

/* Exit 0 = every check passed; 1 = at least one check failed; 2 = setup could
 * not run (no nvcuda, missing export); 124 = deadline. */
static int w1_finish(const char *fixture)
{
    w1_emit("\"event\":\"finish\",\"fixture\":\"%s\",\"failures\":%d", fixture, w1_failures);
    return w1_failures ? 1 : 0;
}

/* ---- nvcuda resolution ---- */

static HMODULE w1_nvcuda;

static void w1_module_identity(HMODULE module, const char *label)
{
    WCHAR path[MAX_PATH] = {0};
    char narrow[MAX_PATH * 3] = {0};
    GetModuleFileNameW(module, path, MAX_PATH);
    WideCharToMultiByte(CP_UTF8, 0, path, -1, narrow, sizeof narrow, NULL, NULL);
    for (char *c = narrow; *c; c++)
        if (*c == '\\')
            *c = '/';
    const IMAGE_DOS_HEADER *dos = (const IMAGE_DOS_HEADER *)module;
    const IMAGE_NT_HEADERS *nt = (const IMAGE_NT_HEADERS *)((const char *)module + dos->e_lfanew);
    const IMAGE_DATA_DIRECTORY *dir = &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT];
    unsigned long names = 0, functions = 0;
    if (dir->VirtualAddress) {
        const IMAGE_EXPORT_DIRECTORY *exp =
            (const IMAGE_EXPORT_DIRECTORY *)((const char *)module + dir->VirtualAddress);
        names = exp->NumberOfNames;
        functions = exp->NumberOfFunctions;
    }
    w1_emit("\"event\":\"module\",\"label\":\"%s\",\"export_names\":%lu,\"export_functions\":%lu,"
            "\"machine\":%u,\"v_path\":\"%s\",\"v_base\":\"%p\"",
            label, names, functions, nt->FileHeader.Machine, narrow, (void *)module);
}

static HMODULE w1_load_nvcuda(void)
{
    const char *name = getenv("W1_NVCUDA");
    if (!name)
        name = "nvcuda.dll";
    w1_nvcuda = LoadLibraryA(name);
    if (!w1_nvcuda) {
        w1_emit("\"event\":\"nvcuda-load\",\"ok\":false,\"v_error\":%lu", (unsigned long)GetLastError());
        return NULL;
    }
    w1_module_identity(w1_nvcuda, "nvcuda");
    return w1_nvcuda;
}

static void *w1_sym(const char *name)
{
    return w1_nvcuda ? (void *)GetProcAddress(w1_nvcuda, name) : NULL;
}

/* Declare a function pointer resolved from nvcuda; missing exports are
 * reported and leave the pointer NULL. */
#define W1_FN(ret, name, args) typedef ret(CUDAAPI *PFN_##name) args; static PFN_##name p_##name
#define W1_RESOLVE(name, exported)                                                           \
    do {                                                                                     \
        p_##name = (PFN_##name)w1_sym(exported);                                             \
        if (!p_##name)                                                                       \
            w1_emit("\"event\":\"missing-export\",\"name\":\"%s\"", exported);               \
    } while (0)

W1_FN(CUresult, cuInit, (unsigned int));
W1_FN(CUresult, cuDriverGetVersion, (int *));

static void w1_resolve_common(void)
{
    W1_RESOLVE(cuInit, "cuInit");
    W1_RESOLVE(cuDriverGetVersion, "cuDriverGetVersion");
}

#define W1_MISSING (-1)
#define CU(name, ...) (p_##name ? p_##name(__VA_ARGS__) : W1_MISSING)

#endif
