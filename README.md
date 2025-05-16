# gh mount

A simple tool to mount a GitHub repository as a local filesystem.

## Requirements

To use this tool, you need to have the following installed:
- [GitHub CLI](https://cli.github.com/)
- FUSE
    - linux [Installing FUSE on Linux](#installing-fuse-on-linux)
    - macOS [Installing FUSE on macOS](#installing-fuse-on-macos)
    - windows: ??

### Installing FUSE on Linux

Many distributions have a working FUSE setup out-of-the-box. However if it is not working for you, you may need to install and configure FUSE manually.

#### on Ubuntu (>= 24.04):

```bash
sudo add-apt-repository universe
sudo apt install libfuse2t64
```
Note: In Ubuntu 24.04, the libfuse2 package was renamed to libfuse2t64.

#### on Ubuntu (>= 22.04):
```bash
sudo add-apt-repository universe
sudo apt install libfuse2
```
Warning: While libfuse2 is OK, do not install the fuse package as of 22.04 or you may break your system. If the fuse package did break your system, you can recover as described here.

#### on Ubuntu (<= 21.10):

```bash
sudo apt install fuse libfuse2
sudo modprobe fuse
sudo groupadd fuse

user="$(whoami)"
sudo usermod -a -G fuse $user
```

### Installing FUSE on macOS

Install this tool and enable it:
- `macfuse`
    - https://github.com/macfuse/macfuse
    - https://macfuse.github.io/


## Installation

```bash
# https://cli.github.com/manual/gh_extension
gh extension list ## check if gh is installed and see the list of extensions

gh extension install victorlpgazolli/gh-mount

```

## Limitations

Since this is a simple tool, it has some limitations:
- It only works with public repositories.
- Shell completion is not available.
- Zsh plugins crushes performance, so it is recommended to use bash. :/

## Usage

```bash
gh auth status # login to your GitHub account

cd $(mktemp -d) # create a temporary directory just for testing :)
mkdir github
gh mount ./github

## Recommended: use bash
bash
## accessing the mounted repository
cd github/victorlpgazolli
ls -la # listing my repositories
cd gh-mount
ls -la # listing the files in this current repository

## unmounting the repository
umount ./github

```