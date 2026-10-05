# Objective
We want to build an app that shows a 3D view of the sky as it would look from the user's current location.

# Tech Stack
HTML5 single page app, Rust, WASM, Bevy game engine

# User Interface
When the user opens the app, ask for permission to use the user's location. If they provide their location, use that to determine where on the earth to show the sky from. If they don't give permission, assume Seattle, WA, USA.

If the user is on mobile, ask for permission to use their accelerometer and compass. If you have those, use them to determine what the user is pointing at, so for example if the user points at a red "star" in the sky and that "star" is actually Mars, Mars should appear in the app.

If the user is not on mobile, allow them to use their mouse to pan the starfield. In both cases, display right ascension and declension in the app so the user knows what they're pointing at and if their compass is correctly calibrated.

Display stars on a black background in the app, with stars at their actual locations in the sky. Use white for stars, and color planets as appropriate. The sun should be yellow and the moon should be white. Display the milky way as a grey band in its actual location.

Planets, major starts (Polaris, Betelgeuse) should be labeleled. Label the milky way as well.

# Star Data
Use open source star location to compute the location of the stars. Display all starts of at least 5 magnitude and above. Use open-source ephemeris data to calculate the location of the planets at the current time.
