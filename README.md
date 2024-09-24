# Hextazy

A coloful __hexadecimal editor__, inspired by [hexyl](https://github.com/sharkdp/hexyl).

![Illustration with all possible bytes](./images/hextazy.png)

## Build

```bash
git clone https://github.com/0xfalafel/hextazy.git
cd hextazy
cargo build
```

## Install

An amd64 linux binary is available: [https://github.com/0xfalafel/hextazy/releases/latest/](https://github.com/0xfalafel/hextazy/releases/latest/).


### Cargo

If you already have rust installed. You can install the app with `cargo`:

```bash
cargo install hextazy
```
If you don't have `cargo` installed. There are installation instructions here [https://doc.rust-lang.org/cargo/getting-started/installation.html](https://doc.rust-lang.org/cargo/getting-started/installation.html).

### Archlinux

You can install [from the AUR](https://aur.archlinux.org/packages/hextazy) using an [AUR helper](https://wiki.archlinux.org/title/AUR_helpers) like so:

```bash
paru -S hextazy
```

### Nix

Hextazy is also available as a [nix package](https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/he/hextazy/package.nix)

```bash
nix-env -i hextazy
```

## Usage

`hextazy` take the file to edit as an argument.

```
Usage: hextazy [file]
```

```bash
hextazy ./test/all_bytes.bin
```

You can edit the file directly. Use `Tab` to switch between the Hexdecimal and Ascii editors.

Once you're done, press __`q`__ or __`Ctrl + C`__ to __exit__.

## Shortcuts

### Core shortcuts

| Key Combination   | Action       |
|-------------------|--------------|
| `Ctrl + Q`        | __Exit__ the app. |
| `Ctrl + C`        | Exit the app without saving. |
| `q`               | Exit the app (in _hex editor_ mode). |
| `Tab`             | Switch between _ascii_ and _hexadecimal_ editor mode. |
| `Ctrl + J`        | Switch between __Insert__ and __Overwrite__ mode. |
| `Ctrl + Z`        | __Undo__ the last write. |
| `Ctrl + S`        | __Save__ your changes. |
| `Del`             | __Delete__ the following byte in __Insert mode__. |
| `:`               | Open the command bar. |
| `Esc`             | Close the command bar. |
| `/`               | Open the search bar. |

### Handy shortcuts

| Key Combination   | Action       |
|-------------------|--------------|
| `Ctrl + →`        | Jump 4 bytes to the right. |
| `Ctrl + ←`        | Jump 4 bytes to the left. |
| `Ctrl + Y`        | __Redo__, cancel the last _undo_. |
| `Ctrl + U`        | __Undo all__ changes. |
| `Backspace`       | __Move left__ / __Undo__ the modification of the __previous byte__. |


### Search

| Key Combination   | Action       |
|-------------------|--------------|
| `/4142`           | Search the hex value `0x4142`, and the ascii string `"4142"`. |
| `n`               | Go to the next search result. |
| `Shift + n`       | Go to the previous search result. |
| `:s/abc`          | Search the _string_ `"acb"`. |
| `:x/4142`         | Search the hex value `0x4142`. |
| `:xi/4142`        | Search the hex value in reverse order: `0x4241`. |

### Commands

The command bar is opened with `:` in the _hexadecimal editor_ mode.

| Command           | Action       |
|-------------------|--------------|
| `:`               | Open the command bar. |
| `:q`              | Close the app. |
| `:0x1234`         | Jump at the address `0x1234`. |
| `:i` or `:insert` | Switch to _insert_ mode. |
| `:o` or `:overwrite` | Switch to _overwrite_ mode. |
| `:hexyl`          | Switch to the _hexyl_ sytle. |
| `:!hexyl`          | Switch to the _default_ sytle. |
