# nuFAT

nuFAT is an experimental filesystem driver to use the FAT32 filesystem in the Linux userspace.

> [!Warning]
> nuFAT is currently in its earliest stages of development and therefor not ready for any real world use.
> Please refer to the `Supported Features` section for the currently supported filesystem operations.

## Set up

To set nuFAT up, please clone the repository.

Make sure that all `fuse` dependencies are installed.

### OpenSUSE Tumbleweed

```sh
sudo zypper in libfuse3-3 fuse3 fuse-devel 
```

### Fedora

```sh
sudo dnf install fuse fuse-devel
```

### Ubuntu

```sh
sudo apt-get install fuse3 libfuse3-dev
```

When everything is installed, refer to the [testing guide](./docs/testing.md) on how to run nuFAT.

## Supported Features

- [ ] List directory entries
- [ ] Read file contents
- [ ] Create new file
- [ ] Create new directory
- [ ] Write content to file
- [ ] Delete file or directory

## Reporting issues

If you encounter an issue while using nuFAT, please [report them](https://github.com/ByteOtter/nuFAT/issues) in the issues section.

## License

nuFAT is licensed under the terms of the [MIT License](./LICENSE).
