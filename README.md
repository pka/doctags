doctags
=======

doctags is a simple document management system based on tags.

doctags is written in Rust and uses the full-text search engine [Tantivy](https://github.com/tantivy-search/tantivy).

Tags
----

Tags are stored in human readable `.doctags.toml` files, supporting network file systems and sharing tags.

Hierarchical tags are preferably separated with a colon like in `lang:de`. In search expressions,
tags are identified by a colon prefix (e.g. `:lang:de`).

Installation
------------

    cargo install -f --path doctags-cli

    # https://github.com/zargony/fuse-rs
    sudo apt-get install fuse
    sudo apt-get install libfuse-dev pkg-config

    cargo install -f --path doctagsfs

Add .doctags.toml to your global git ignore file:

    echo .doctags.toml >> ~/.gitignore

Usage
-----

    doctags help

Create index:

    doctags index $HOME/code

Search matching paths:

    doctags search t-rex
    doctags search 't-rex README'

Tagging examples:

    for d in *t-rex*; do doctags tag $d project:t-rex; done

    find . -type d -name .git -exec doctags tag --recursive false {}/.. gitrepo \;

Update index:

    doctags reindex

Search tagged paths:

    doctags search ':project:t-rex .toml'
    doctags search -l 0 ':gitrepo *'

Use terminal UI:

    doctags ui

Mount virtual file system:

    doctagsfs default /mnt/doctags

Unmount:

    sudo fusermount -u /mnt/doctags

Mount from fstab:

    sudo ln -s $HOME/.cargo/bin/doctagsfs /sbin/mount.doctags
    echo "default   /mnt/doctags    doctags   noauto,ro,user,exec    0 0" | sudo tee -a /etc/fstab
    mount /mnt/doctags


Using Alt-c from a shell
------------------------

Changing the directory within an application doesn't change the state of the calling shell.

For [Nushell](https://www.nushell.sh/) you can define an alias:

    alias dt [] { doctags ui --printcd true | cd $it }

For Bash you can use a function:

```
BIN=$HOME/.cargo/bin/doctags
function dt {
    f=$(mktemp)
    (
    set +e
    $BIN ui --outcmd "$f" "$@"
    code=$?
    if [ "$code" != 0 ]; then
        rm -f "$f"
        exit "$code"
    fi
    )
    code=$?
    if [ "$code" != 0 ]; then
    return "$code"
    fi
    d=$(<"$f")
    rm -f "$f"
    eval "$d"
}
```


Troubleshooting
---------------

If `doctacs` panics with a message like "Unknown error while starting watching directory ...", then tantivy
has reached the kernel inotify watch limit. Check with:

    sudo sysctl fs.inotify.max_user_watches

and increase it temporarely with:

    sudo sysctl fs.inotify.max_user_watches=16384
