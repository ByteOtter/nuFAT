# nuFAT

nuFAT is an experimental filesystem driver to use the FAT32 filesystem in the Linux userspace.

## Set up

To set nuFAT up, please clone the repository.

Make sure that all `fuse` dependencies are installed.

### OpenSUSE Tumbleweed

```sh
sudo zypper in libfuse3-3 fuse3 fuse-devel 
```

### Ubuntu

```sh
sudo apt-get install fuse3 libfuse3-dev
```

When everything is installed, refer to the [testing guide](./docs/testing.md) on how to run nuFAT.
