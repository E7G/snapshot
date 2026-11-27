# Rebuild the Mi Pad 2 7.2 camera variants

This workflow keeps the 15 fps display-only preview used on the physical
tablet separate from the experiments. It does not install anything, alter
the default camera launcher, or test a camera device.

| Variant | Source |
| --- | --- |
| preview15 (default) | 18ce2f64348d521993a322e0244a49cdeaaa0b9f, the 15 fps local installation recipe |
| preview-conversion | 1c8204ae39fb34bca7cf7b670efabcb991fcc87b, display scaling after rate limiting |
| capture-pool | 1e13361f8dc7955d13876bee46be56415b028854, independent larger native buffer pool |
| direct-nv12 | bc843a84661e5816524768017070107df1446d77, opt-in unqualified recording experiment |

```bash
gh workflow run build-mipad2.yml -R E7G/snapshot \
  --ref codex/mipad2-72-snapshot-replay -f variant=preview15
```

The job checks out the exact source commit, downloads the crates in Cargo.lock,
builds the matching Meson profile, runs the camera preview tests and checks the
archive SHA-256. The receipt records the installed binary hash and Arch dependency
versions for comparison. The rolling base image and dependency repositories do
not promise byte-identical packages. The physical installation recorded binary
SHA-256 e237f69ca6282f41fdda39285ebc7b1f823718658528f4df5eac469117bb760f.
