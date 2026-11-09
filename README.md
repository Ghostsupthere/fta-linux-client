# From The Ashes: native Linux client

A native Linux client (no Wine) for **From The Ashes**

This repository holds the installer and launcher, and the **[Releases](https://github.com/Ghostsupthere/fta-linux-client/releases/latest)**
page has the downloads. The client's **source code** is in
**[fta-wd-ix-linux-native](https://github.com/Ghostsupthere/fta-wd-ix-linux-native)** (a C++ reconstruction of the 3.3.5a
client with the Linux platform layer and the WarcraftXL ports); the feature-by-feature status is in its
[docs/WXL-NATIVE-PORT.md](https://github.com/Ghostsupthere/fta-wd-ix-linux-native/blob/main/docs/WXL-NATIVE-PORT.md).

> The client is not at proven parity with the Windows client. Some features are confirmed in game, others are
> implemented but not yet checked, and a few are off by default. 

## Requirements

- A From The Ashes client folder
- Linux x86-64 with a Vulkan-capable GPU and driver
- SDL3 (3.2 or newer), libcurl, ALSA and the PulseAudio client libraries. The installer checks for these and
  can install them (see the distribution notes below). glibc, the C++ runtime, xvidcore and dxvk-native are
  bundled, so your distribution's glibc version does not matter.

## Install

Download the latest release files from the **Releases** page.

### One-click install

Download `FTA-Linux-Installer.run` and **double-click it**. If your file manager asks, choose *Run*. If it
only opens it as text, right-click it, choose *Properties*, then *Permissions*, and tick *Allow executing*;
or run `bash FTA-Linux-Installer.run` in a terminal.

It opens a terminal, finds your From The Ashes client folder (or asks you to pick it), installs into it, offers
to install any missing system libraries (it asks for your password), adds **From The Ashes (Linux)** to your
applications menu (it opens the launcher) and offers to open the launcher. Run it again at any time to update.

**Addon fixes.** The installer carries a few small fixes for addons in the base client (ElvUI chat, the ElvUI bag
quest icon and mail skin, Questie's server bridge and world map icons, and the Ashen Bot Upgrade tooltip). It lists each file it would change, with a one-line reason, and
asks first. A file is replaced only if it is the version the fix was made for; otherwise the patch is applied if it
fits (this needs the `patch` tool), and otherwise the file is left alone. Originals are kept in
`FTA-Linux/addon-fix-backup/`. Use `--no-fixes` to skip them, or `--fixes PATH` to apply another fixes folder or
`.tar.gz`. These are Lua files shared with the Windows client in the same folder, so Windows gets the fixes too.
Up to v0.2.6 they were a separate `FTA-Addon-Fixes.tar.gz` download; from v0.2.7 the single `.run` is all you need.

### The launcher

The launcher comes with the installer (there is no separate download). Open **From The Ashes (Linux)** from your
applications menu, or run `FTA-Linux/launcher.sh` in your client folder.

- **Play**: starts the game. When a newer release is out it says *Update ready*; the Update button downloads the
  installer from this project's release page, checks it, asks before installing, installs it and restarts. The
  *What's new* panel shows the release notes.
- **Game**: realm address (written to `realmlist.wtf` and `Config.wtf`), windowed, vsync, resolution and
  anti-aliasing.
- **Settings**: the WarcraftXL and performance options as checkboxes, with presets and a Safe mode, and a Debugging
  section (Lua errors, logging switches). **Apply** saves them and keeps the previous files in
  `FTA-Linux/settings-backup/`. Nothing is saved while the game is running.
- **Diagnostics**: checks the system, graphics driver, game files and libraries, and copies or saves a report to
  attach to a bug report (it holds no account data).

`FTA-Linux/play.sh` still starts the game without the launcher (the menu entry's *Play directly* action does the
same).

### Manual install

1. Open a terminal **in your client folder** (the one with `Wow.exe`). In most file managers: right-click an
   empty spot in that folder, then *Open Terminal Here*.
2. Unpack the archive into a temporary folder **and run the installer** with this one line (change the path if
   the file is not in `~/Downloads`):

   ```
   mkdir fta-linux-install && tar -xzf ~/Downloads/fta-linux-client.tar.gz -C fta-linux-install && ./fta-linux-install/install.sh
   ```

   Unpacking alone installs nothing. When it finishes, an `FTA-Linux` folder exists in your client folder.
3. Install any missing system libraries, then check and play, still in the client folder:

   ```
   ./FTA-Linux/install-deps.sh --install
   ./FTA-Linux/play.sh --check
   ./FTA-Linux/play.sh
   ```

4. Optional: `rm -r fta-linux-install`.

`install.sh` installs into the folder you run it from; to install into another folder, give its path:
`./install.sh /path/to/client`. Always start the game with the `FTA-Linux/play.sh` in your client folder, never
with anything under `fta-linux-install`.

### Checking a download

`FTA-Linux-SHA256SUMS.txt` lists the checksums. Compare with `sha256sum FTA-Linux-Installer.run`. The
installer also checks itself and reports a damaged copy.

## Distribution notes

SDL3 is the one dependency older releases lack. `FTA-Linux/install-deps.sh` reports what is missing and the
exact command for your distribution; `install-deps.sh --install` runs it (asks first, uses sudo; `--yes` skips
the question).

- **Arch, CachyOS, Manjaro:** everything is in the repositories. NVIDIA users add `nvidia-utils`.
- **Debian 13 and Ubuntu 25.04 or newer:** everything is in the repositories (`libsdl3-0`). NVIDIA users add the
  proprietary driver (`nvidia-driver`, or Ubuntu's driver tool).
- **Ubuntu 22.04 / 24.04 and Debian 12:** no SDL3 package. Upgrade, or build SDL 3.2 or newer into
  `/usr/local`, which the launcher searches:

  ```
  sudo apt-get install build-essential cmake ninja-build git pkg-config libx11-dev libxext-dev libxcursor-dev \
      libxi-dev libxfixes-dev libxrandr-dev libxss-dev libxkbcommon-dev libwayland-dev wayland-protocols \
      libdrm-dev libgbm-dev libegl-dev libasound2-dev libpulse-dev libudev-dev libdecor-0-dev libvulkan-dev
  git clone --depth 1 --branch release-3.2.x https://github.com/libsdl-org/SDL.git
  cmake -S SDL -B SDL/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DSDL_TESTS=OFF -DSDL_EXAMPLES=OFF
  cmake --build SDL/build && sudo cmake --install SDL/build
  ./FTA-Linux/install-deps.sh --install      # the rest
  ```

- **Fedora:** SDL3 is packaged from Fedora 42 (`SDL3`); on older releases build it as above. NVIDIA users add
  the driver from RPM Fusion (`akmod-nvidia`).
- **openSUSE:** Tumbleweed has `libSDL3-0`; releases without it need the source build above.

Tested: Arch/CachyOS (played), Debian 13 and Ubuntu 24.04 with a source-built SDL 3.2 (library check and engine
start in containers without a GPU). The Fedora and openSUSE package names come from those distributions'
package lists and have not been run; if a name is wrong, install the library that `play.sh --check` reports.
The maintainer's SDL3 is 3.4.x; 3.2.x loads but has not been played.
The launcher has only been run on Arch/CachyOS under Wayland (KDE). It is untested on X11 and on other
distributions; if it does not open, `FTA-Linux/play.sh` starts the game without it.

## Settings

Native features are switched in the client's `WarcraftXL.cfg` (the file the Windows client reads) or by an
environment variable of the same name, which wins. Missing keys use built-in defaults.
The launcher's Settings tab edits the same file for you.

| Key | Effect |
|---|---|
| `WXL_MODERN_ASSETS` | Convert modern M2/BLP/WMO assets at load |
| `WXL_COLLECTION_ARMOR` | Collection armor models |
| `WXL_VISUAL_ENVIRONMENT`, `WXL_MODERN_WATER`, `WXL_GOD_RAYS`, `WXL_LOCAL_LIGHTS` | Water, atmosphere and lighting |
| `WXL_GRASS_MOTION` | Grass sway and parting around the player |
| `WXL_SURFACE_LIGHTS` | Torches and lamps light terrain, models and buildings (off unless set to `1`) |
| `WXL_QUEST_RENDERING`, `WXL_UNIT_OVERLAYS` | Quest markers and objective rings; target outline and danger guides (off) |

## Troubleshooting

- **"Unknown command ./FTA-Linux/install-deps.sh"**: the installer has not run yet. Run
  `./fta-linux-install/install.sh` from your client folder first.
- **"this is the unpacked installer, not the installed client"**: you ran `play.sh` from inside
  `fta-linux-install`. Run `install.sh`, then use `./FTA-Linux/play.sh` in the client folder.
- **"the installer file is damaged"**: the download or copy is incomplete. Download it again and compare
  `sha256sum` with `FTA-Linux-SHA256SUMS.txt`.
- **Anything wrong with an install or update**: the installer keeps a log at `Logs/fta-linux-install.log` in your
  client folder (it appends one section per run, and prints the path at the end). It records the folder it chose and
  the checksum of `Wow64` before and after, so it shows whether the right client folder was updated. Attach it when
  reporting. If the run stopped before a folder was chosen, the log is `~/.local/state/fta-linux-client/install.log`.
  `sha256sum Wow64` in the client folder should match the one in the log's "installed Wow64" line.
- **Missing libraries**: run `FTA-Linux/install-deps.sh` (see the distribution notes).
- **The launcher does not open**: run `FTA-Linux/launcher.sh` in a terminal to see why. It needs a desktop with a
  compositor (Wayland or X11) and `fontconfig`; `install-deps.sh` reports it. `FTA-Linux/play.sh` starts the game
  without the launcher.
- **"GitHub's server did not answer (HTTP 504)" when updating**: GitHub's download servers are sometimes slow for a
  file published a few minutes ago. Press *Try again* after a minute.
- **A crash**: the report is in `Errors/` and the launch log in `Logs/native-launch-*.log`, both in your client
  folder. Attach both when reporting.

## Uninstall

Delete `Wow64` and the `FTA-Linux` folder from your client folder, and
`~/.local/share/applications/fta-linux-client.desktop` if you added the menu shortcut. The offline shader packs
in `Utils/WarcraftXL/visuals/` are harmless to keep.

## Releasing

The binary parts are built in the [source tree](https://github.com/Ghostsupthere/fta-wd-ix-linux-native) and packaged here. Set `RELEASE_TAG` (for example `RELEASE_TAG=v0.2.10`) when packaging, so the launcher can tell which release is installed.

## Layout

| Path | Contents |
|---|---|
| `overlay/` | `install.sh` and `FTA-Linux/play.sh`, `FTA-Linux/launcher.sh`, `FTA-Linux/install-deps.sh`: what gets installed into a client folder |
| `installer/installer-header.sh` | The one-click installer's script part (the payload is appended at build time) |
| `tools/package.sh` | Builds the release files from a payload |
| `tools/make-addon-fixes.sh` | Builds the addon-fixes folder (`fixes.manifest`, patches, fixed files) that `package.sh` bundles into the installer |
| `launcher/` | The launcher (Rust and Slint): update check, Play, game and WarcraftXL settings, diagnostics |
| `tools/build-launcher.sh` | Builds the launcher against the game's glibc (needs Nix) |
