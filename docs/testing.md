# Testing the filesystem

After you have [created a sample disk image](./creating_sample_image.md), you can now execute `nuFAT` and see if it works.

To do so, you need to provide a mounting point. You can either do this by [creating a proper mounting point](#proper) 
at `mnt/myfusefat/` or [creating a phony mounting point](#phony)

# Creating a proper mounting point(#proper)

You can create a "proper" mounting point in `/mnt/` using

```sh
sudo mkdir -p /mnt/myfusefat
```

> [!Warning]
> Doing this can cause `nuFAT` to crash as accessing `/mnt/` requires elevated privileges.
> I therefore recomment you go and [use a phony mounting point](#phony)

# Creating a phony mounting point(#phony)

You can also specify a directory as mount location which you as a regular user have access to without the need for any privileges.

To do so, create a new directory and pass it to `nuFAT`.

```sh
mdkir ./myfatfs/

cargo run -- $DISK_IMAGE_PATH ./myfatfs/
```

The program should then execute.

# Accessing the filesystem

No matter what mount option you use, you can access the filesystem by opening a terminal and `cd`-ing in it.

```sh
cd ./myfatfs/
```

You can then try performing file operations as you are used to.
If you have followed my guide on how to create a disk image, the file system is empty as it was created using `/dev/zero`.
