# Sky Viewer

A single-page 3D view of the night sky built with Rust, WebAssembly, and Bevy.
At startup, an existing browser location grant is used automatically; otherwise
choose whether to use location or enter coordinates. The coordinate fields start
with a default location. The sky remains behind a loading screen until the
location choice is resolved and the first mesh is built, so the initial sky uses
the final selected coordinates.
On phones and tablets, the optional compass view follows device orientation;
on desktop, drag the sky with the mouse. Press F2 to view sky viewer information,
F3 to toggle render diagnostics, and F4 to toggle label categories or search for
an object and smoothly point the view toward it. Initial sky meshes build
incrementally; frames longer than 100 ms are logged to the browser console.
Slow sky-update stages are timed separately, and browsers that support the Long
Tasks API also report long-task attribution to help distinguish application
work from browser or rendering stalls.

## Run locally

Install Rust and Cargo, then run:

```sh
./launch.sh [--debug|--release]
```

The default is `--debug`, which includes debug symbols. Use `--release` for a
fully optimized build without debug symbols.

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

## GitHub Pages deployment

The GitHub Actions workflow builds the WebAssembly app and deploys it to GitHub
Pages on pushes to `main` (or when run manually), caching Rust build artifacts
and Trunk to speed up later runs. The workflow requests Pages enablement
automatically. The published URL is `https://ebrown32.github.io/sky-viewer-demo/`.
Publishing a private repository with GitHub Pages requires an account plan
that supports Pages for private repositories.

## Sky data and calculations

`assets/stars.tsv` contains every HYG v3.8 catalog entry with apparent
magnitude 5.0 or brighter. The data is distributed under CC BY-SA 2.5; see
`assets/README.md` and `assets/HYG-LICENSE.html` for attribution and license
details. Proper motion is applied from the J2000 catalog epoch.

Planet positions use the JPL approximate heliocentric orbital elements for
1800–2050, solved with Kepler's equation
(<https://ssd.jpl.nasa.gov/planets/approx_pos.html>). The Moon uses a low-precision
geocentric orbital model with the principal periodic corrections, and its
displayed phase follows the Sun's direction for the current date and time.
These are intended for a visual sky guide, not navigation or astrometry.

The star coordinates are transformed from equatorial to local horizontal
coordinates using the observer's latitude, longitude, and local sidereal time.
Right ascension and declination shown in the interface describe the direction
at the center of the view.
