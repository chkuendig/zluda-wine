// SPDX-License-Identifier: MIT OR Apache-2.0
// ZLUDA_WINE_INTEROP=1 enables D3D10 and D3D11 CPU-copy interop; default off.
pub fn enabled()->bool {std::env::var_os("ZLUDA_WINE_INTEROP").is_some_and(|v|v=="1")}
