# ZLUDA Wine bridge

This is the Wine-side `nvcuda` bridge for running Windows CUDA applications and
games with [ZLUDA](https://github.com/chkuendig/ZLUDA) on Linux. Windows programs
call its `nvcuda.dll`; the bridge passes those calls through Wine's Unix-call
interface to ZLUDA's Linux CUDA library.

The 64-bit DLL forwards typed CUDA calls. The 32-bit DLL translates pointers and
handles through Wine's modern WoW64 interface to the same 64-bit Unix module.
Callbacks return to Windows threads. CUDA coverage depends on both the bridge
and the backend; unsupported entry points return CUDA errors.

## Building

Build on x86_64 Linux. You need Rust **1.99.0**, Python 3.9 or newer, a native C
compiler and binutils, and the x86_64 and i686 MinGW-w64 compilers, headers and
static libraries. On Debian/Ubuntu the cross-compiler packages are
`gcc-mingw-w64-x86-64`, `gcc-mingw-w64-i686`, `mingw-w64-x86-64-dev` and
`mingw-w64-i686-dev`.

```sh
rustup toolchain install 1.99.0 --profile minimal
rustup target add --toolchain 1.99.0 \
  x86_64-unknown-linux-gnu x86_64-pc-windows-gnu i686-pc-windows-gnu
export PATH="$HOME/.rustup/toolchains/1.99.0-x86_64-unknown-linux-gnu/bin:$PATH"
```

The bridge also needs a **matching Wine build tree**, including `winebuild`,
source and generated headers, and `dlls/ntdll/ntdll.so`. The current Valve target
is Wine from Proton **11.0-2e**, commit
`e60892cc2398b0a83c332a2078c7b85e47bdb42b`, with the callback-handler restoration
patch used by the Proton fork. An arbitrary system Wine development package
cannot substitute for this tree. The supported Wine identities are in
`wine/zluda_wine/tools/wine-providers.json`.

The reference build is
[`make/rules-zluda.mk`](https://github.com/chkuendig/Proton/blob/proton-11.0-zluda/make/rules-zluda.mk)
in the Valve Proton fork. It builds Wine and prepares the SDK automatically.
`WINE_SOURCE` must be the patched Proton Wine build copy after applying
`patches/wine/`, including the callback-handler restoration patch. A pristine
Wine checkout is rejected by the SDK helper. Given the fork's checkout and its
completed x86_64 Wine build, prepare a fresh SDK (replace the example paths):

```sh
export PROTON_SOURCE=/absolute/path/to/Proton
export WINE_SOURCE=/absolute/path/to/patched/proton/wine/source
export WINE_OBJECTS=/absolute/path/to/wine-x86_64-build
export WINE_SDK_ROOT=/absolute/path/to/new/wine-sdk
python3 "$PROTON_SOURCE/make/zluda-wine-sdk.py" \
  "$WINE_SOURCE" "$WINE_OBJECTS" "$WINE_SDK_ROOT"
export ZLUDA_WINE_ABI_RECEIPT="$WINE_SDK_ROOT/ABI-RECEIPT.json"
export ZLUDA_WINE_ABI_RECEIPT_SHA256="$(sha256sum "$ZLUDA_WINE_ABI_RECEIPT" | cut -d' ' -f1)"
```

The SDK helper checks the Wine source identities and records the hashes of the
built SDK tools and callback provider. The bridge checks that receipt before
building. Keep the receipt with its SDK.

Obtain a ZLUDA source checkout for code generation. For example, commit
`5220554de03a471de81a895e7b0dce3d10cf6f1c` of `chkuendig/ZLUDA` contains the
expected declarations. Other commits work when the six declaration inputs have
the same SHA256 hashes in `wine/zluda_wine/tools/backend_source.py`. No backend
Cargo dependencies or ROCm SDK are needed to compile the bridge itself.

From this repository's root:

```sh
export ZLUDA_WINE_BACKEND_SOURCE=/absolute/path/to/ZLUDA
export ZLUDA_WINE_BUILD_ROOT=/absolute/path/to/new/bridge-build
export ZLUDA_WINE_SOURCE_ID="$(git rev-parse HEAD)"
export SOURCE_DATE_EPOCH="$(git show -s --format=%ct HEAD)"
JOBS=3 bash wine/zluda_wine/build-bridge.sh
```

`ZLUDA_WINE_BUILD_ROOT` must not exist yet. `JOBS` defaults to 3; set it to any
positive integer to choose the build parallelism. The script builds all three modules
and puts them under its `artifacts/` directory:

| Output | Purpose |
| --- | --- |
| `x86_64-windows/nvcuda.dll` | 64-bit Windows CUDA entry points |
| `i386-windows/nvcuda.dll` | 32-bit Windows CUDA entry points |
| `x86_64-unix/nvcuda.so` | Native Linux bridge for both Windows architectures |

There is no Unix32 module: this bridge uses modern WoW64. Build and install the
three modules together against the same Wine build. `SHA256SUMS` and
`BUILD-CONTRACT.json` describe the outputs.

## Using it in a Wine prefix

Use the matching patched Proton Wine installation described above. **32-bit
applications require new-style WoW64**, selected with `WINEARCH=wow64` before
creating the prefix; this mode also runs 64-bit applications. Classic WoW64 is
not supported. Use a fresh prefix rather than reusing a classic WoW64 prefix.

Build or install ZLUDA separately, including its ROCm runtime dependencies.
A ZLUDA Cargo build produces **Linux x86_64** `libnvcuda.so`, which the Proton
fork installs as `libcuda.so.1`. Use that Linux library even when the Windows
program is 32-bit.

Keep the bridge modules in a directory with Wine's architecture subdirectories:

```sh
export BRIDGE_HOME="$HOME/.local/lib/zluda-wine"
mkdir -p "$BRIDGE_HOME"
cp -a "$ZLUDA_WINE_BUILD_ROOT/artifacts/x86_64-windows" \
      "$ZLUDA_WINE_BUILD_ROOT/artifacts/i386-windows" \
      "$ZLUDA_WINE_BUILD_ROOT/artifacts/x86_64-unix" "$BRIDGE_HOME/"
export WINEPREFIX="$HOME/.wine-zluda"
export WINEARCH=wow64
wineboot -u
cp "$BRIDGE_HOME/x86_64-windows/nvcuda.dll" "$WINEPREFIX/drive_c/windows/system32/nvcuda.dll"
cp "$BRIDGE_HOME/i386-windows/nvcuda.dll" "$WINEPREFIX/drive_c/windows/syswow64/nvcuda.dll"
export WINEDLLPATH="$BRIDGE_HOME${WINEDLLPATH:+:$WINEDLLPATH}"
export WINEDLLOVERRIDES="nvcuda=b${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
export PROTON_ENABLE_ZLUDA=1
export ZLUDA_WINE_BACKEND_PATH=/absolute/path/to/ZLUDA/target/release/libnvcuda.so
```

Use the matching Wine's `wine` and `wineboot` executables on `PATH`. `system32`
contains the 64-bit DLL; `syswow64` contains the 32-bit DLL. `WINEDLLPATH` lets
Wine find the Unix module, and `nvcuda=b` selects the Wine builtin bridge. Replace
any existing `nvcuda` override with that value rather than specifying it twice.
The `PROTON_ENABLE_ZLUDA` switch is required for manual Wine use too.
`ZLUDA_WINE_BACKEND_PATH` must name an absolute path to the real Linux library.
If its dependencies are outside the system loader paths, add their directories
to `LD_LIBRARY_PATH` before launching Wine.

On a Steam Deck, the ROCm configuration used with this backend also needs:

```sh
export HSA_OVERRIDE_GFX_VERSION=10.3.0
```

Run a Windows CUDA sample or game using the same environment:

```sh
WINEDEBUG=+loaddll wine /absolute/path/to/cuda-sample.exe
```

Check that Wine loads this `nvcuda.dll`, that `cuInit(0)` returns `CUDA_SUCCESS`,
and that device enumeration finds the AMD GPU. A device-query sample should
print its name; running a CUDA kernel then checks execution as well as loading.
For a 32-bit program, `ZLUDA_WINE_CALL_LOG` can name a writable log path for
optional call tracing.

GPU PhysX also needs patched dxvk-nvapi, `DXVK_NVAPI_ALLOW_OTHER_DRIVERS=1`
and `DXVK_ENABLE_NVAPI=1`; see [Notes](#notes).

The [Valve Proton fork](https://github.com/chkuendig/Proton/tree/proton-11.0-zluda)
integrates the backend, bridge and matching Wine automatically. Enable it with
`PROTON_ENABLE_ZLUDA=1` in the game's environment.

## Tests and license

The hosted workflow checks Linux, PE64 and PE32 declarations, runs native Rust
and Python tests, and exercises an actual PE32 LUID/stack probe on Windows.
The complete module build also runs in the Proton fork's CI. For local unit
tests, set `ZLUDA_WINE_BACKEND_SOURCE` as above and
`ZLUDA_WINE_DECLARATIONS_ONLY=1`, then run:

```sh
python3 -m unittest discover -s wine/zluda_wine/tests -p 'test_*.py'
python3 wine/zluda_wine/tests/run_native_tests.py
```

Copyright © Christian Kündig and contributors. Licensed under MIT OR Apache-2.0;
see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

## Notes

### Wine steamclient fault fix

The [Wine patch](patches/wine/0001-ntdll-restore-thread-state-after-steamclient-fault.patch)
restores thread state before redirecting a steamclient fault. Proton's loader
forces `WINESTEAMNOEXEC=1` for several CEG titles, including Mafia II. This maps
the game's `steamclient.dll` `.text` non-executable, so calls fault and
`segv_handler()` redirects RIP to the lsteamclient export. That early return
skips `leave_handler()`, unlike the handler's other early returns.

For a 32-bit thread under new WoW64, only `leave_handler()` reloads the 32-bit
`%fs` selector. Without it, the redirected code runs with a null `%fs` and
crashes on its first TEB access, as Mafia II can at startup. 64-bit and classic
WoW64 processes are unaffected. The bridge requires new WoW64 for 32-bit games,
so affected titles need this fix.

The fix adds one call to `leave_handler()` in Proton Wine's
[handler](https://github.com/ValveSoftware/wine/blob/proton_11.0/dlls/ntdll/unix/signal_x86_64.c#L2731).

### dxvk-nvapi patches for PhysX

Apply these patches in order to dxvk-nvapi
[commit `f9fdf268`](https://github.com/jp7677/dxvk-nvapi/commit/f9fdf268),
the Valve Proton 11.0 pin:

1. [Bound V1 topology writes](patches/dxvk-nvapi/0001-nvapi-gpu-Bound-CUDA-topology-V1-writes.patch).
2. [Discover CUDA compute GPUs through nvcuda](patches/dxvk-nvapi/0002-nvapi-Discover-CUDA-compute-GPUs-through-nvcuda.patch).
3. [Support V2 topology for GPUs confirmed by CUDA](patches/dxvk-nvapi/0003-nvapi-gpu-Allow-CUDA-topology-V2-for-GPUs-confirmed-.patch).
4. [Add the private PhysX topology query](patches/dxvk-nvapi/0004-nvapi-Add-undocumented-CUDA-topology-entry-point-0x3.patch).
5. [Allow other drivers in every topology test section](patches/dxvk-nvapi/0005-tests-Allow-other-drivers-in-every-CUDA-topology-sec.patch).
6. [Match CUDA devices by PCI location](patches/dxvk-nvapi/0006-nvapi-Match-CUDA-devices-by-PCI-location-instead-of-.patch).
7. [Restore test adapter LUIDs](patches/dxvk-nvapi/0007-tests-Give-CUDA-topology-test-adapters-a-LUID-again.patch).

CUDA-capable GPUs are discovered through `nvcuda.dll` and matched to DXGI
adapters by PCI domain, bus and device. The V2 and private PhysX queries use
driver-owned storage that remains valid until `NvAPI_Unload`.

Install and enable the patched dxvk-nvapi for the prefix, and launch with
`DXVK_NVAPI_ALLOW_OTHER_DRIVERS=1` and `DXVK_ENABLE_NVAPI=1`. Proton also sets
`PROTON_FORCE_NVAPI=1`. These patches do not require DXVK to report an NVIDIA
vendor ID. The [Valve Proton fork](https://github.com/chkuendig/Proton/tree/proton-11.0-zluda)
applies the Wine patch and this series automatically. For a manual build,
apply the Wine patch to Proton Wine and the dxvk-nvapi patches in the order above.
