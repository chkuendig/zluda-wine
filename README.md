# ZLUDA Wine CUDA component

This independent MIT OR Apache-2.0 component supplies the PE64, PE32 and Unix
Wine CUDA bridge. Its clean-room production source was extracted from public
ZLUDA `8a8e3b76`; existing notices, owner grants and human credits are retained.
See `wine/zluda_wine/LICENSES/PROVENANCE.md` and the root license texts.

Set `ZLUDA_WINE_BACKEND_SOURCE` to an absolute checkout of the backend commit
in `BACKEND-SOURCE.json`. Generation byte-checks its six permissive source
inputs. There are no backend Cargo dependencies or duplicated declarations.
Proton pins this component and the backend independently and supplies the
matching prepared Wine SDK and digest-pinned ABI receipt. Build all three
modules together using `wine/zluda_wine/build-bridge.sh`; do not mix targets.
The production source capsule includes the admitted backend input closure and
can rebuild all three targets offline without the original checkouts.

Optional PE32 `ZLUDA_WINE_CALL_LOG` tracing is disabled by default. The CPU
fixtures cover module identity, PE32 widths, ABI dispatch and lifecycle cleanup.
Packaging builds and admits them from the selected bridge source and Wine SDK.
