/* SPDX-License-Identifier: MIT OR Apache-2.0 */
#include "w1.h"

static void bridge_identity(unsigned expected_exports)
{
    w1_resolve_common();
    const IMAGE_DOS_HEADER *dos = (const IMAGE_DOS_HEADER *)w1_nvcuda;
    const IMAGE_NT_HEADERS *nt = (const IMAGE_NT_HEADERS *)((const char *)w1_nvcuda + dos->e_lfanew);
    const IMAGE_DATA_DIRECTORY *directory = &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT];
    unsigned names = directory->VirtualAddress ?
        ((const IMAGE_EXPORT_DIRECTORY *)((const char *)w1_nvcuda + directory->VirtualAddress))->NumberOfNames : 0;
    w1_check("export-count", names == expected_exports, "\"expected\":%u,\"actual\":%u",
             expected_exports, names);
    int version = 0;
    CUresult result = CU(cuDriverGetVersion, &version);
    w1_check("cuDriverGetVersion", result == CUDA_SUCCESS && version > 0,
             "\"status\":%d,\"version\":%d", result, version);
    result = CU(cuInit, 0);
    w1_check("cuInit", result == CUDA_SUCCESS, "\"status\":%d", result);
}

int main(int argc, char **argv)
{
    const char *expect = "on";
    unsigned exports = 0;
    for (int i = 1; i + 1 < argc; i += 2) {
        if (!strcmp(argv[i], "--expect"))
            expect = argv[i + 1];
        else if (!strcmp(argv[i], "--exports"))
            exports = (unsigned)strtoul(argv[i + 1], NULL, 10);
    }
    w1_start("identity", 120000);
    w1_emit("\"event\":\"expect\",\"mode\":\"%s\"", expect);
    if (!w1_load_nvcuda()) {
        w1_check("nvcuda-load", 0, "\"mode\":\"%s\"", expect);
        return w1_finish("identity");
    }
    if (!strcmp(expect, "off")) {
        int any = w1_sym("cuInit") || w1_sym("cuGetExportTable") || w1_sym("cuDriverGetVersion");
        w1_check("off-no-cuda-exports", !any, "\"cuInit\":%s", w1_sym("cuInit") ? "true" : "false");
    } else {
        bridge_identity(exports);
    }
    return w1_finish("identity");
}
