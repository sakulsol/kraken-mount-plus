<p align="center"><img src="docs/logo.png" width="128" alt="Kraken Mount Plus"></p>

# Kraken Mount Plus

Kraken Mount Plus is [Kraken Mount](https://github.com/quer3q/kraken-mount) by quer3q with every image
format of [ShadowMount+](https://github.com/drakmor/ShadowMountPlus) added (`.exfat`, `.ffpkg`, `.ffpfs`,
`.ffpfsc`) and a per-game switch to turn off ampr-emu-ng for games that crash with it. All credit for
Kraken Mount itself goes to its authors; the changes in this fork are listed in [What is new](#what-is-new).
These changes are also offered upstream; if they are merged, use the original Kraken Mount.

A PS5 game mounter. It finds games on the internal SSD, the M.2 drive and USB drives.

**Supported formats**: debug FPKG (`.pkg`), game folders (plain, or packed with zstd or LZ4) and
images as made by ShadowMount+'s tools: `.exfat`, `.ffpkg` (UFS), `.ffpfs` (PFS) and `.ffpfsc`
(compressed PFS container); mounted read-only.
For large, demanding games, use the zstd pack format: it performs better than FPKG.

## How to

1. Download `kraken-mount.elf` from [Releases](https://github.com/sakulsol/kraken-mount-plus/releases)
   and send it to your jailbroken PS5 with the ELF loader (elfldr, port 9021) or payload-manager:
   `nc -w 3 <ps5-ip> 9021 < kraken-mount.elf`. kstuff must be loaded, and ShadowMount+ must not be
   running: Kraken Mount waits until both are true.
2. Put each game in a scan root, or in a folder directly inside one (for example
   `/data/homebrew/rpg/<game>`; deeper folders are not scanned, and on a drive root only the root itself):
   - internal SSD: `/data/homebrew`, `/data/etaHEN/games`
   - M.2 or extended drive (`/mnt/ext0`, `/mnt/ext1`): `homebrew`, `etaHEN/games` or the drive root
   - USB drive (`/mnt/usb0` to `/mnt/usb7`): `homebrew`, `etaHEN/games` or the drive root
3. Each game gets a "<name> registered" notification and its own home-screen tile.

[ampr-emu-ng](https://github.com/quer3q/ampr-emu-ng) comes bundled and is installed into
`/data/krakenmount/ampr/` at start. To run another build, put its `libSceAmpr.sprx` there; it is
kept until you delete it.

A game that needs the DLC or PlayGo emulator and ships none gets the bundled one in its fakelib
(setting `add_emulators`, on by default); a game's own copy is kept, unless it is a known older
release of the bundled one, which is upgraded. The PlayGo stub can be turned off per game in the
game's details. Details:
[DLC emulator](docs/dlcemu.md), [PlayGo stub](docs/playgo.md).

Caches live in `/data/krakenmount/caches/<TITLE_ID>`, settings in
`/data/krakenmount/config.toml` and the log in `/data/krakenmount/kraken.log`.

To build it yourself: `ps5/build.sh` (needs Docker), output `target/ps5/kraken-mount.elf`.

## What is new

Compared with Kraken Mount 0.3.1:

- **Image files**: `.exfat`, `.ffpkg` (UFS), `.ffpfs` (PFS) and `.ffpfsc` (compressed PFS container)
  are found like any other game, attached through `/dev/lvdctl` and mounted read-only under
  `/mnt/krakenmount`, with the same LVD profiles as ShadowMount+. They show an IMG chip and their
  extension in the web UI; ejecting a game releases its image.
- **ampr-emu-ng per game**: a switch in the game's details (setting `ampr_off`) keeps the bundled
  ampr-emu-ng out of that game's fakelib. Some games (for example Days Gone and Hogwarts Legacy)
  only start with it off.
- **Issue reports** name the image file a game was mounted from.

Tested on a PS5 with firmware 12.60.

## Thanks

- [drakmor/ShadowMountPlus](https://github.com/drakmor/ShadowMountPlus): home-screen
  registration, the fakelib sandbox overlay and backports are ported from it.
- [drakmor/dlc_emu](https://github.com/drakmor/dlc_emu): the bundled DLC emulator
  (`libSceAppContent.sprx`, `libSceNpEntitlementAccess.sprx`, `libSceGameUpdate.sprx`), added to a
  game's fakelib when the game has a `dlc_emu.ini` at its root; a known older release the game ships
  is upgraded, any other copy is kept.
- [drakmor/pgo_stub](https://github.com/drakmor/pgo_stub): the bundled PlayGo stub
  (`libScePlayGo.sprx`), added to a game's fakelib when its `eboot.bin` imports `libScePlayGo` and the
  game ships none.
- [ps5-payload-sdk](https://github.com/ps5-payload-dev/sdk): the PS5 toolchain.

## License

GPL-3.0-or-later (`LICENSE`). Third-party notices are in `THIRD_PARTY_NOTICES`.
