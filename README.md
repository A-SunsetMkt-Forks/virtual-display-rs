# Virtual Display Driver

[![Build](https://github.com/MolotovCherry/virtual-display-rs/actions/workflows/build.yml/badge.svg?branch=master&event=push)](https://github.com/MolotovCherry/virtual-display-rs/actions/workflows/build.yml) [![GitHub release (with filter)](https://img.shields.io/github/v/release/MolotovCherry/virtual-display-rs)](https://github.com/MolotovCherry/virtual-display-rs/releases)

This is a Windows driver made in Rust which creates a virtual desktop.

It has many uses, such as:
- A private virtual desktop for VR use
- For remote desktops
- Getting a higher resolution (or higher refresh rate) display when you don't have a physical one on-hand (though note you can only use it in software/VR)
- Other uses? Let me know!

Support: Windows 10 x64 +

_Note about downloads:  
Chrome and Windows likes to vet software by amount of downloads. If you receive a download warning, or a smartscreen warning when you try to run it, this is because of Chrome/Windows respectively, not the software. If for any reason you are unsure, the source code is in the repo, and you can build it yourself from scratch using the [build instructions](https://github.com/MolotovCherry/virtual-display-rs#how-to-build)._

## Features
- Multiple monitors (up to 10)
- Multiple resolutions per monitor
- Multiple refresh rates per resolution
- App to configure them all, disable all/individual monitors

https://github.com/MolotovCherry/virtual-display-rs/assets/13651622/4a244e40-65d2-4c99-91f7-4e8b352e3ebe

# How to install
- Go to the [releases](https://github.com/MolotovCherry/virtual-display-rs/releases) section for the latest driver.
- Download (you may receive a warning, just press accept)
- Install certificate (see below section)
- Open `Device Manager`
- - Click on any item
  - Click on `Action` menu item -> `Add legacy hardware` -> `Next`
  - Click `Install the hardware that I manually select from a list (Advanced)`
  - Click `Next` (`Show All Devices` will be selected)
  - Click `Have Disk`
  - Browse to the location of the folder, press `Ok`, and keep clicking the `Next` buttons

### Installing the certificate
The certificate needs installation for Windows to accept the driver
- In your downloaded zip, double click on the file `DriverCertificate.cer`
- A window will popup with a `Install Certificate` button (click it)
- Select `Local Machine`
- Select `Place All Certificates in the following store`, click `Browse` and select `Trusted Root Certification Authorities`
- Cick `Next` and `Finish`

# Updating
- Open `Device Manager`
- Under the `Display` section, find the `Virtual Display` driver and double click
- Click the `Driver` tab and the `Update Driver` button
- Click `Browse my computer for drivers`, browse for the right location, and click `Next`

# Using the app
Please see the [wiki](https://github.com/MolotovCherry/virtual-display-rs/wiki/Virtual-Display-Driver-Control) for instructions on using the app.

# How to build
- Download and install [Visual Studio](https://visualstudio.microsoft.com/downloads/) (use the 2022 edition)
- Select and install the `Desktop development with C++` workload as well as Windows SDK
- Install the [WDK](https://learn.microsoft.com/en-us/windows-hardware/drivers/download-the-wdk)
- Install [`cargo-make`](https://github.com/sagiegurari/cargo-make) if you don't have it
- You can build it with `cargo make -p dev build` (debug) or `cargo make -p prod build` (release)
- ... Or, fork my project and build it with github actions

### Debugging
To see panic messages and other information, download [DebugViewPP](https://github.com/CobaltFusion/DebugViewPP), run it, click on `Log`->`Capture Global Win32` (note, this requires DebugViewPP be run with admin permissions)

# Contributions
All contributions are welcome! If you have any questions, feel free to post in the project [Discussion](https://github.com/MolotovCherry/virtual-display-rs/discussions) section
