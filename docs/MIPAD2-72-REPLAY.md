# Rebuild the Mi Pad 2 camera variants

The live LPAT31 tablet was rechecked on 2026-11-28: its default launcher
executes `/usr/local/lib/mipad2-snapshot-native-ae-r6/bin/snapshot`. The observed
binary SHA-256 is `f16944d2b075af0ad6aa75e465036a5520fb00a74b0180d4bf433968f6a6b3d6`.
The 15 fps binary has a historical install receipt, but it is not the active
launcher target. These builds do not install or activate anything.

| Variant | Source |
| --- | --- |
| native-ae-r6 (current default) | db794b2fe85c72dd77710880bbc45ca3909ace1a, explicit native pipeline shutdown |
| preview15 | 18ce2f64348d521993a322e0244a49cdeaaa0b9f, separately qualified 15 fps display preview |
| preview-conversion | 1c8204ae39fb34bca7cf7b670efabcb991fcc87b, display scaling after rate limiting |
| capture-pool | 1e13361f8dc7955d13876bee46be56415b028854, separate larger buffer pool |
| direct-nv12 | bc843a84661e5816524768017070107df1446d77, opt-in experiment |

```bash
gh workflow run build-mipad2.yml -R E7G/snapshot \
  --ref codex/mipad2-72-snapshot-replay -f variant=native-ae-r6
```

The workflow pins the source and Cargo.lock; the receipt records the
dependencies, image digest, archive and binary hashes. A new build can differ
at the byte level from the installed binary. Host tests do not replace a new
front/rear-camera or clean-close test on a physical tablet.
