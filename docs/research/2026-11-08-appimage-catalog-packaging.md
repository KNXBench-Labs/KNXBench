# AppImage catalog launcher permissions and glibc baseline

Observed 8 November 2026. Packaging-only scope: no KNX protocol, project format,
commissioning permission, new release or replacement of published assets.

## Published artifact

The catalog's [PR #11305](https://github.com/AppImage/appimage.github.io/pull/11305)
reports a launcher execution failure in
[run 42463912251](https://github.com/AppImage/appimage.github.io/actions/runs/42463912251).
The inspected input is the actual published
[`v0.1.0-alpha.19` AppImage](https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.19),
not a newly rebuilt substitute:

- Asset `KNXBench_0.1.0-alpha.19_amd64.AppImage`, 97,008,120 bytes.
- SHA-256 `3c35ba25cbbf569352cf13106bed626f3ed4b7443d7dc34d85b661ad72b6d5df`,
  matched against that release's published `SHA256SUMS`.
- Extracted launcher modes: `AppRun` 0755; `AppRun.wrapped` **0770**.
- The catalog's exact error is `AppRun.wrapped: Permission denied`.
  The missing window is downstream of this failure; the generic offline warning
  does not establish an application-level network dependency.

Direct SquashFS inode inspection independently confirms owner/group **0/0**
and those launcher modes. The published payload has 136 directories at 0755
and one at 0777. Its type-2 runtime extraction nevertheless creates all 137
directories at 0700: runtime extraction is not a faithful permission oracle.
Use `--appimage-offset` and `unsquashfs -o <offset>` for mode verification and
cross-user startup, without correcting modes after extraction.

## Why the mode survives bundling

[Tauri CLI 2.11.4's AppImage bundler](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs)
copies its cached `AppRun-<architecture>` into the AppDir before invoking
linuxdeploy with the GTK input plugin and AppImage output plugin. Its executable
helper creates tools with restricted permissions. Correcting only the outer
AppImage executable, or testing as the owner, does not establish that the
wrapped inner launcher is executable by another user.

## Separate glibc finding

The catalog also reports `GLIBC_2.43`. Inspection of undefined dynamic ELF
symbols confirms that this is a real packaged requirement, not an inference
from the inspecting machine's libc. Six bundled libraries require 2.43:

| Packaged file | Example undefined symbol at `GLIBC_2.43` |
| --- | --- |
| `usr/lib/libglib-2.0.so.0` | `free_sized`, `free_aligned_sized` |
| `usr/lib/libgnutls.so.30` | `__memset_explicit_chk` |
| `usr/lib/libjavascriptcoregtk-4.1.so.0` | `sqrtf` |
| `usr/lib/libjxl.so.0.11` | `sqrtf`, `remainder` |
| `usr/lib/liblcms2.so.2` | `sqrtf` |
| `usr/lib/libwebkit2gtk-4.1.so.0` | `coshf`, `atan2f` |

The wrapped launcher itself requires at most glibc 2.3. The desktop executable's
highest undefined glibc version is 2.39, including weak references. The complete
image scan covers the runtime plus 144 ELF payload files.

The [tag build, run 41330363960](https://github.com/KNXBench-Labs/KNXBench/actions/runs/41330363960)
actually resolved `ubuntu-latest` to **ubuntu-26.04**. Its dependency-install log
contains the matching WebKitGTK 2.52.6 package for that distribution. Thus an
unversioned runner label raised the bundled-library baseline without a project
source change. The permission fix alone cannot lower those ELF requirements.

## Implementation boundary

Pin the AppImage build job to `ubuntu-24.04`, and check the finished image's
undefined glibc symbol versions and numeric ELF version-needs entries against
a maximum of **2.39**. Scan every actual
ELF file, including helper processes, shared libraries and the outer runtime;
ignore version definitions and non-ELF resources. This is an ABI ceiling, not a
claim that every distribution with that libc is supported. The AppImage still
uses the host's libc and graphics stack.

Normalize the AppDir before packaging and reject unreadable/unsearchable payloads
or non-world-executable launchers in the finished image. Run an additional
startup smoke with another UID, private application data and only loopback
networking; the embedded desktop server needs loopback, not Internet access.
A visible window and frontend readiness must replace an alive-only claim.

Automatic AppImage builds remain restricted to release tags. Manual dispatch is
reserved for intentional release-preparation checks, never every branch commit
or pull request. A new catalog `/retest` is appropriate only after a corrected
release asset is public; source delivery does not repair the old asset.

Runtime validation and delivery evidence belong in the package's status record;
this note records the source/artifact findings, not an unexecuted success claim.
