# SPDX-License-Identifier: MIT OR Apache-2.0
"""Validate the CUDA declarations used to generate the bridge."""
import hashlib
import os
from pathlib import Path


SOURCE_INPUTS = {
    "cuda_macros/src/cuda.rs": "fe4c9326ccb34c894793f99e8f3d403e7f6acd440d6468ab3796003b8db2648a",
    "cuda_types/src/cuda.rs": "b6adddfb2635d66003d6a282cb96a1dbc86cdf9b20d2fed19378c9fc1466a7b6",
    "cuda_types/src/dark_api.rs": "3b7f25038ecb8208f7784755c16ff6d655a3aeadf3d2116e342522dfd331941b",
    "dark_api/src/lib.rs": "69d27adc2465e27b0cb31d5e67306899ad2f3f4fa2a3d53b76cc49f022afb2dc",
    "zluda/src/impl/mod.rs": "4b83a64de2a74ae2e98c8357813aca71f7436c72a9ecd871aa6e4d2baa97ca7e",
    "zluda/src/lib.rs": "47770e378d251cf21545524681d0c6241e0c53d5e35681be96da7764b49132fe"
}


def backend_source():
    value = os.environ.get('ZLUDA_WINE_BACKEND_SOURCE')
    if not value or not Path(value).is_absolute():
        raise ValueError('ZLUDA_WINE_BACKEND_SOURCE must name an absolute pinned backend source root')
    root = Path(value).resolve(strict=True)
    for name, expected in SOURCE_INPUTS.items():
        path = (root / name).resolve(strict=True)
        if not path.is_relative_to(root) or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Pinned backend source differs: ' + name)
    return root


if __name__ == "__main__":
    root = backend_source()
    for name in SOURCE_INPUTS:
        print("cargo:rerun-if-changed=" + str(root / name))
