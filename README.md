# Sky Viewer

A single-page 3D view of the night sky built with Rust, WebAssembly, and Bevy.
It uses the browser's location when permitted and otherwise starts from Seattle.
On phones and tablets, the optional compass view follows device orientation;
on desktop, drag the sky with the mouse. Press F2 to view sky viewer information
and F3 to toggle render diagnostics, including draw calls and a frame-timing graph.

## Run locally

Install Rust and Cargo, then run:

```sh
./launch.sh
```

The launcher installs Trunk with Cargo if needed and uses `apt-get` to install
the WebAssembly Rust standard library and linker when they are missing. On
Debian-based systems, package installation requires apt privileges.

Open the local HTTPS/localhost URL printed by Trunk. Browser geolocation and
motion sensors require a secure context; `localhost` qualifies. To check the
astronomy calculations without building the Bevy app:

```sh
rustc --edition=2021 --test src/astro.rs -o /tmp/sky-astro-tests
/tmp/sky-astro-tests
```

Cargo is configured to select registry crate versions only after a 30-day
cooldown (`.cargo/config.toml`). Use a Cargo release that supports
`registry.global-min-publish-age` when resolving or updating dependencies.

## Sky data and calculations

`assets/stars.tsv` contains every HYG v3.8 catalog entry with apparent
magnitude 5.0 or brighter. The data is distributed under CC BY-SA 2.5; see
`assets/README.md` and `assets/HYG-LICENSE.html` for attribution and license
details. Proper motion is applied from the J2000 catalog epoch.

Planet positions use the JPL approximate heliocentric orbital elements for
1800–2050, solved with Kepler's equation
(<https://ssd.jpl.nasa.gov/planets/approx_pos.html>). The Moon uses a low-precision
geocentric orbital model with the principal periodic corrections. These are
intended for a visual sky guide, not navigation or astrometry.

The star coordinates are transformed from equatorial to local horizontal
coordinates using the observer's latitude, longitude, and local sidereal time.
Right ascension and declination shown in the interface describe the direction
at the center of the view.
