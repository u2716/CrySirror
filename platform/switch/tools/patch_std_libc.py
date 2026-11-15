#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Point a copied rust-src library workspace at the patched libc (idempotent).

Current nightlies resolve the -Zbuild-std workspace separately from the
project, so neither the project's nor the library's [patch] reaches std's
libc. The Switch build therefore copies the toolchain's library/ and rewrites
every member's libc dependency to the devkitA64-layout libc checkout.
usage: patch_std_libc.py LIBRARY_DIR LIBC_DIR_RELATIVE_TO_A_MEMBER
"""
import glob
import os
import re
import sys

library, libc = sys.argv[1], sys.argv[2]
changed = 0
for manifest in glob.glob(os.path.join(library, "*", "Cargo.toml")):
    text = open(manifest).read()
    new = re.sub(r'^libc = \{ version = "[^"]*",', 'libc = { path = "%s",' % libc, text, flags=re.M)
    if new != text:
        open(manifest, "w").write(new)
        changed += 1
print(f"patched libc in {changed} std manifests")
