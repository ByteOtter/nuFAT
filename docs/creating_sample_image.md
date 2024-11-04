# Creating a sample disk image for testing

> [!Note]
> This guide does only work on Linux systems.

## 1. Creating a bare image using `dd`

```sh
dd if=/dev/zero of=disk.img bs=1M count=10
```

- `/dev/zero` is a dummy device which only returns zeroes
- `of=disk.img` is the name of the resulting image file
- `bs=1M` the blocksize (set to 1Megabyte)
- `count=10` Number of blocks (here 10) resulting in 10MB of space.

## 2. Formatting the image with FAT

```sh
sudo mkfs.vfat disk.img
```

This should format your raw image into the FAT file format.
