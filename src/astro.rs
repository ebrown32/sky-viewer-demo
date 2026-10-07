use std::f64::consts::PI;

const DEG: f64 = PI / 180.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Equatorial {
    pub ra_deg: f64,
    pub dec_deg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Horizontal {
    pub azimuth_deg: f64,
    pub altitude_deg: f64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Planet {
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

#[derive(Clone, Copy)]
struct Elements {
    a: f64,
    a_rate: f64,
    e: f64,
    e_rate: f64,
    inclination: f64,
    inclination_rate: f64,
    mean_longitude: f64,
    longitude_rate: f64,
    perihelion: f64,
    perihelion_rate: f64,
    node: f64,
    node_rate: f64,
}

impl Planet {
    pub const ALL: [Self; 7] = [
        Self::Mercury,
        Self::Venus,
        Self::Mars,
        Self::Jupiter,
        Self::Saturn,
        Self::Uranus,
        Self::Neptune,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Mercury => "Mercury",
            Self::Venus => "Venus",
            Self::Mars => "Mars",
            Self::Jupiter => "Jupiter",
            Self::Saturn => "Saturn",
            Self::Uranus => "Uranus",
            Self::Neptune => "Neptune",
        }
    }

    fn elements(self) -> Elements {
        match self {
            Self::Mercury => Elements {
                a: 0.38709927,
                a_rate: 0.00000037,
                e: 0.20563593,
                e_rate: 0.00001906,
                inclination: 7.00497902,
                inclination_rate: -0.00594749,
                mean_longitude: 252.25032350,
                longitude_rate: 149472.67411175,
                perihelion: 77.45779628,
                perihelion_rate: 0.16047689,
                node: 48.33076593,
                node_rate: -0.12534081,
            },
            Self::Venus => Elements {
                a: 0.72333566,
                a_rate: 0.00000390,
                e: 0.00677672,
                e_rate: -0.00004107,
                inclination: 3.39467605,
                inclination_rate: -0.00078890,
                mean_longitude: 181.97909950,
                longitude_rate: 58517.81538729,
                perihelion: 131.60246718,
                perihelion_rate: 0.00268329,
                node: 76.67984255,
                node_rate: -0.27769418,
            },
            Self::Mars => Elements {
                a: 1.52371034,
                a_rate: 0.00001847,
                e: 0.09339410,
                e_rate: 0.00007882,
                inclination: 1.84969142,
                inclination_rate: -0.00813131,
                mean_longitude: -4.55343205,
                longitude_rate: 19140.30268499,
                perihelion: -23.94362959,
                perihelion_rate: 0.44441088,
                node: 49.55953891,
                node_rate: -0.29257343,
            },
            Self::Jupiter => Elements {
                a: 5.20288700,
                a_rate: -0.00011607,
                e: 0.04838624,
                e_rate: -0.00013253,
                inclination: 1.30439695,
                inclination_rate: -0.00183714,
                mean_longitude: 34.39644051,
                longitude_rate: 3034.74612775,
                perihelion: 14.72847983,
                perihelion_rate: 0.21252668,
                node: 100.47390909,
                node_rate: 0.20469106,
            },
            Self::Saturn => Elements {
                a: 9.53667594,
                a_rate: -0.00125060,
                e: 0.05386179,
                e_rate: -0.00050991,
                inclination: 2.48599187,
                inclination_rate: 0.00193609,
                mean_longitude: 49.95424423,
                longitude_rate: 1222.49362201,
                perihelion: 92.59887831,
                perihelion_rate: -0.41897216,
                node: 113.66242448,
                node_rate: -0.28867794,
            },
            Self::Uranus => Elements {
                a: 19.18916464,
                a_rate: -0.00196176,
                e: 0.04725744,
                e_rate: -0.00004397,
                inclination: 0.77263783,
                inclination_rate: -0.00242939,
                mean_longitude: 313.23810451,
                longitude_rate: 428.48202785,
                perihelion: 170.95427630,
                perihelion_rate: 0.40805281,
                node: 74.01692503,
                node_rate: 0.04240589,
            },
            Self::Neptune => Elements {
                a: 30.06992276,
                a_rate: 0.00026291,
                e: 0.00859048,
                e_rate: 0.00005105,
                inclination: 1.77004347,
                inclination_rate: 0.00035372,
                mean_longitude: -55.12002969,
                longitude_rate: 218.45945325,
                perihelion: 44.96476227,
                perihelion_rate: -0.32241464,
                node: 131.78422574,
                node_rate: -0.00508664,
            },
        }
    }
}

pub fn julian_date(unix_seconds: f64) -> f64 {
    unix_seconds / 86_400.0 + 2_440_587.5
}

pub fn local_sidereal_time_deg(julian_day: f64, longitude_deg: f64) -> f64 {
    let centuries = (julian_day - 2_451_545.0) / 36_525.0;
    normalize_degrees(
        280.460_618_37
            + 360.985_647_366_29 * (julian_day - 2_451_545.0)
            + 0.000_387_933 * centuries * centuries
            - centuries * centuries * centuries / 38_710_000.0
            + longitude_deg,
    )
}

pub fn equatorial_to_horizontal(
    equatorial: Equatorial,
    latitude_deg: f64,
    sidereal_time_deg: f64,
) -> Horizontal {
    let latitude = latitude_deg * DEG;
    let declination = equatorial.dec_deg * DEG;
    let hour_angle = normalize_signed_degrees(sidereal_time_deg - equatorial.ra_deg) * DEG;
    let sin_altitude = latitude.sin() * declination.sin()
        + latitude.cos() * declination.cos() * hour_angle.cos();
    let altitude = sin_altitude.clamp(-1.0, 1.0).asin();
    let azimuth = (-declination.cos() * hour_angle.sin()).atan2(
        declination.sin() * latitude.cos()
            - declination.cos() * latitude.sin() * hour_angle.cos(),
    );
    Horizontal {
        azimuth_deg: normalize_degrees(azimuth / DEG),
        altitude_deg: altitude / DEG,
    }
}

pub fn horizontal_to_equatorial(
    horizontal: Horizontal,
    latitude_deg: f64,
    sidereal_time_deg: f64,
) -> Equatorial {
    let latitude = latitude_deg * DEG;
    let altitude = horizontal.altitude_deg * DEG;
    let azimuth = horizontal.azimuth_deg * DEG;
    let sin_declination = altitude.sin() * latitude.sin()
        + altitude.cos() * latitude.cos() * azimuth.cos();
    let declination = sin_declination.clamp(-1.0, 1.0).asin();
    let hour_angle = (-altitude.cos() * azimuth.sin()).atan2(
        altitude.sin() * latitude.cos()
            - altitude.cos() * latitude.sin() * azimuth.cos(),
    );
    Equatorial {
        ra_deg: normalize_degrees(sidereal_time_deg - hour_angle / DEG),
        dec_deg: declination / DEG,
    }
}

pub fn proper_motion(
    ra_hours: f64,
    dec_deg: f64,
    pm_ra_mas_year: f64,
    pm_dec_mas_year: f64,
    years_since_epoch: f64,
) -> Equatorial {
    let dec = dec_deg * DEG;
    let cos_dec = dec.cos().abs().max(1.0e-6);
    Equatorial {
        ra_deg: normalize_degrees(
            ra_hours * 15.0 + pm_ra_mas_year * years_since_epoch / (3_600_000.0 * cos_dec),
        ),
        dec_deg: (dec_deg + pm_dec_mas_year * years_since_epoch / 3_600_000.0)
            .clamp(-90.0, 90.0),
    }
}

pub fn galactic_to_equatorial(longitude_deg: f64, latitude_deg: f64) -> Equatorial {
    let longitude = longitude_deg * DEG;
    let latitude = latitude_deg * DEG;
    let xg = latitude.cos() * longitude.cos();
    let yg = latitude.cos() * longitude.sin();
    let zg = latitude.sin();

    let x = -0.054_875_560_4 * xg + 0.494_109_427_9 * yg - 0.867_666_149_0 * zg;
    let y = -0.873_437_090_2 * xg - 0.444_829_630_0 * yg - 0.198_076_373_4 * zg;
    let z = -0.483_835_015_5 * xg + 0.746_982_244_5 * yg + 0.455_983_776_2 * zg;
    Equatorial {
        ra_deg: normalize_degrees(y.atan2(x) / DEG),
        dec_deg: z.clamp(-1.0, 1.0).asin() / DEG,
    }
}

pub fn sun_equatorial(julian_day: f64) -> Equatorial {
    let earth = heliocentric_position(earth_elements(), julian_day);
    ecliptic_to_equatorial(-earth.0, -earth.1, -earth.2)
}

pub fn planet_equatorial(planet: Planet, julian_day: f64) -> Equatorial {
    let earth = heliocentric_position(earth_elements(), julian_day);
    let body = heliocentric_position(planet.elements(), julian_day);
    ecliptic_to_equatorial(body.0 - earth.0, body.1 - earth.1, body.2 - earth.2)
}

pub fn moon_equatorial(julian_day: f64) -> Equatorial {
    let days = julian_day - 2_451_543.5;
    let node = (125.1228 - 0.052_953_808_3 * days) * DEG;
    let inclination = 5.1454 * DEG;
    let periapsis = (318.0634 + 0.164_357_322_3 * days) * DEG;
    let eccentricity = 0.0549;
    let mean_anomaly = normalize_degrees(115.3654 + 13.064_992_950_9 * days) * DEG;
    let eccentric_anomaly = solve_kepler(mean_anomaly, eccentricity);
    let xv = 60.2666 * (eccentric_anomaly.cos() - eccentricity);
    let yv = 60.2666 * (1.0 - eccentricity * eccentricity).sqrt() * eccentric_anomaly.sin();
    let true_anomaly = yv.atan2(xv);
    let radius = xv.hypot(yv);
    let argument = true_anomaly + periapsis;
    let x = radius * (node.cos() * argument.cos() - node.sin() * argument.sin() * inclination.cos());
    let y = radius * (node.sin() * argument.cos() + node.cos() * argument.sin() * inclination.cos());
    let z = radius * argument.sin() * inclination.sin();
    let mut longitude = y.atan2(x) / DEG;
    let mut latitude = z.atan2(x.hypot(y)) / DEG;

    let sun_mean_anomaly = normalize_degrees(356.0470 + 0.985_600_258_5 * days) * DEG;
    let sun_longitude =
        normalize_degrees(282.9404 + 4.709_35e-5 * days + sun_mean_anomaly / DEG);
    let mean_lunar_longitude =
        normalize_degrees(node / DEG + periapsis / DEG + mean_anomaly / DEG);
    let elongation = normalize_signed_degrees(mean_lunar_longitude - sun_longitude) * DEG;
    let mean_moon = mean_anomaly;
    let argument_latitude =
        normalize_signed_degrees(mean_lunar_longitude - node / DEG) * DEG;
    longitude += -1.274 * (mean_moon - 2.0 * elongation).sin()
        + 0.658 * (2.0 * elongation).sin()
        - 0.186 * sun_mean_anomaly.sin()
        - 0.059 * (2.0 * mean_moon - 2.0 * elongation).sin()
        - 0.057 * (mean_moon - 2.0 * elongation + sun_mean_anomaly).sin()
        + 0.053 * (mean_moon + 2.0 * elongation).sin()
        + 0.046 * (2.0 * elongation - sun_mean_anomaly).sin()
        + 0.041 * (mean_moon - sun_mean_anomaly).sin()
        - 0.035 * elongation.sin()
        - 0.031 * (mean_moon + sun_mean_anomaly).sin()
        - 0.015 * (2.0 * argument_latitude - 2.0 * elongation).sin()
        + 0.011 * (mean_moon - 4.0 * elongation).sin();
    latitude += -0.173 * (argument_latitude - 2.0 * elongation).sin()
        - 0.055 * (mean_moon - argument_latitude - 2.0 * elongation).sin()
        - 0.046 * (mean_moon + argument_latitude - 2.0 * elongation).sin()
        + 0.033 * (argument_latitude + 2.0 * elongation).sin()
        + 0.017 * (2.0 * mean_moon + argument_latitude).sin();

    let longitude = longitude * DEG;
    let latitude = latitude * DEG;
    let cos_latitude = latitude.cos();
    ecliptic_to_equatorial(
        cos_latitude * longitude.cos(),
        cos_latitude * longitude.sin(),
        latitude.sin(),
    )
}

fn earth_elements() -> Elements {
    Elements {
        a: 1.00000261,
        a_rate: 0.00000562,
        e: 0.01671123,
        e_rate: -0.00004392,
        inclination: -0.00001531,
        inclination_rate: -0.01294668,
        mean_longitude: 100.46457166,
        longitude_rate: 35999.37244981,
        perihelion: 102.93768193,
        perihelion_rate: 0.32327364,
        node: 0.0,
        node_rate: 0.0,
    }
}

fn heliocentric_position(elements: Elements, julian_day: f64) -> (f64, f64, f64) {
    let centuries = (julian_day - 2_451_545.0) / 36_525.0;
    let a = elements.a + elements.a_rate * centuries;
    let eccentricity = elements.e + elements.e_rate * centuries;
    let inclination = (elements.inclination + elements.inclination_rate * centuries) * DEG;
    let mean_longitude = elements.mean_longitude + elements.longitude_rate * centuries;
    let perihelion = elements.perihelion + elements.perihelion_rate * centuries;
    let node = (elements.node + elements.node_rate * centuries) * DEG;
    let argument_perihelion = (perihelion - (elements.node + elements.node_rate * centuries)) * DEG;
    let mean_anomaly = normalize_signed_degrees(mean_longitude - perihelion) * DEG;
    let eccentric_anomaly = solve_kepler(mean_anomaly, eccentricity);
    let xv = a * (eccentric_anomaly.cos() - eccentricity);
    let yv = a * (1.0 - eccentricity * eccentricity).sqrt() * eccentric_anomaly.sin();
    let true_anomaly = yv.atan2(xv);
    let radius = xv.hypot(yv);
    let argument = true_anomaly + argument_perihelion;
    (
        radius * (node.cos() * argument.cos() - node.sin() * argument.sin() * inclination.cos()),
        radius * (node.sin() * argument.cos() + node.cos() * argument.sin() * inclination.cos()),
        radius * argument.sin() * inclination.sin(),
    )
}

fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mut eccentric_anomaly = mean_anomaly + eccentricity * mean_anomaly.sin();
    for _ in 0..8 {
        let delta = (eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - mean_anomaly)
            / (1.0 - eccentricity * eccentric_anomaly.cos());
        eccentric_anomaly -= delta;
        if delta.abs() < 1.0e-12 {
            break;
        }
    }
    eccentric_anomaly
}

fn ecliptic_to_equatorial(x: f64, y: f64, z: f64) -> Equatorial {
    let obliquity = 23.439_291_111 * DEG;
    let equatorial_y = y * obliquity.cos() - z * obliquity.sin();
    let equatorial_z = y * obliquity.sin() + z * obliquity.cos();
    Equatorial {
        ra_deg: normalize_degrees(equatorial_y.atan2(x) / DEG),
        dec_deg: equatorial_z.atan2(x.hypot(equatorial_y)) / DEG,
    }
}

fn normalize_degrees(angle: f64) -> f64 {
    angle.rem_euclid(360.0)
}

fn normalize_signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equatorial_and_horizontal_coordinates_round_trip() {
        let equatorial = Equatorial {
            ra_deg: 123.456,
            dec_deg: -22.75,
        };
        let sidereal_time = 201.25;
        let horizontal = equatorial_to_horizontal(equatorial, 47.6, sidereal_time);
        let actual = horizontal_to_equatorial(horizontal, 47.6, sidereal_time);

        assert!((actual.ra_deg - equatorial.ra_deg).abs() < 1.0e-9);
        assert!((actual.dec_deg - equatorial.dec_deg).abs() < 1.0e-9);
    }

    #[test]
    fn zenith_has_observer_declination_and_local_sidereal_right_ascension() {
        let zenith = horizontal_to_equatorial(
            Horizontal {
                azimuth_deg: 0.0,
                altitude_deg: 90.0,
            },
            47.6,
            123.4,
        );
        assert!((zenith.ra_deg - 123.4).abs() < 1.0e-9);
        assert!((zenith.dec_deg - 47.6).abs() < 1.0e-9);
    }

    #[test]
    fn north_celestial_pole_is_over_north_at_the_observers_latitude() {
        let pole = equatorial_to_horizontal(
            Equatorial {
                ra_deg: 0.0,
                dec_deg: 90.0,
            },
            47.6,
            123.4,
        );

        assert!(pole.azimuth_deg.min(360.0 - pole.azimuth_deg) < 1.0e-9);
        assert!((pole.altitude_deg - 47.6).abs() < 1.0e-9);
    }

    #[test]
    fn polaris_is_near_the_north_horizon_direction_not_the_zenith() {
        let polaris = equatorial_to_horizontal(
            Equatorial {
                ra_deg: 37.954_560_67,
                dec_deg: 89.264_108_97,
            },
            47.6062,
            37.954_560_67,
        );

        assert!(polaris.azimuth_deg < 1.0 || polaris.azimuth_deg > 359.0);
        assert!((polaris.altitude_deg - 47.6062).abs() < 1.0);
        assert!(polaris.altitude_deg < 90.0);
    }

    #[test]
    fn celestial_poles_follow_latitude_independently_of_time_and_longitude() {
        for latitude in [-80.0, -47.6, 0.0, 30.0, 47.6, 80.0] {
            for longitude in [-180.0, -122.3, 0.0, 120.0, 180.0] {
                for day in [2_451_545.0, 2_461_000.5, 2_461_000.75] {
                    let sidereal_time = local_sidereal_time_deg(day, longitude);
                    for (declination, azimuth, altitude) in
                        [(90.0, 0.0, latitude), (-90.0, 180.0, -latitude)]
                    {
                        let pole = equatorial_to_horizontal(
                            Equatorial { ra_deg: 37.95, dec_deg: declination },
                            latitude,
                            sidereal_time,
                        );
                        assert!(normalize_signed_degrees(pole.azimuth_deg - azimuth).abs() < 1.0e-9);
                        assert!((pole.altitude_deg - altitude).abs() < 1.0e-9);
                    }
                }
            }
        }
    }

    #[test]
    fn equatorial_stars_rise_in_the_east_and_set_in_the_west() {
        let star = Equatorial { ra_deg: 123.4, dec_deg: 0.0 };
        for latitude in [-60.0, 0.0, 47.6, 60.0] {
            for (hour_angle, azimuth) in [(-90.0, 90.0), (90.0, 270.0)] {
                let direction = equatorial_to_horizontal(star, latitude, star.ra_deg + hour_angle);
                assert!((direction.altitude_deg).abs() < 1.0e-9);
                assert!((direction.azimuth_deg - azimuth).abs() < 1.0e-9);
            }
        }
    }

    #[test]
    fn horizon_round_trips_across_hemispheres_and_sidereal_times() {
        for latitude in [-80.0, -47.6, 0.0, 47.6, 80.0] {
            for sidereal_time in [0.0, 123.4, 359.9] {
                for heading in [0.0, 45.0, 90.0, 180.0, 270.0, 359.9] {
                    for altitude in [-85.0, -30.0, 0.0, 30.0, 85.0] {
                        let horizontal = Horizontal { azimuth_deg: heading, altitude_deg: altitude };
                        let equatorial = horizontal_to_equatorial(horizontal, latitude, sidereal_time);
                        let actual = equatorial_to_horizontal(equatorial, latitude, sidereal_time);
                        assert!(normalize_signed_degrees(actual.azimuth_deg - heading).abs() < 1.0e-8);
                        assert!((actual.altitude_deg - altitude).abs() < 1.0e-8);
                    }
                }
            }
        }
    }

    #[test]
    fn galactic_center_and_pole_have_expected_equatorial_coordinates() {
        let center = galactic_to_equatorial(0.0, 0.0);
        let north_pole = galactic_to_equatorial(0.0, 90.0);

        assert!((center.ra_deg - 266.405).abs() < 0.01);
        assert!((center.dec_deg + 28.936).abs() < 0.01);
        assert!((north_pole.ra_deg - 192.859).abs() < 0.01);
        assert!((north_pole.dec_deg - 27.128).abs() < 0.01);
    }

    #[test]
    fn proper_motion_keeps_coordinates_in_valid_ranges() {
        let star = proper_motion(23.99, 89.0, 1200.0, -400.0, 26.0);
        assert!((0.0..360.0).contains(&star.ra_deg));
        assert!((-90.0..=90.0).contains(&star.dec_deg));
    }

    #[test]
    fn j2000_sun_is_near_the_expected_direction() {
        let sun = sun_equatorial(2_451_545.0);
        assert!((sun.ra_deg - 281.28).abs() < 0.2);
        assert!((sun.dec_deg + 23.03).abs() < 0.2);
    }

    #[test]
    fn all_planet_and_moon_positions_are_valid() {
        for planet in Planet::ALL {
            let position = planet_equatorial(planet, 2_461_000.5);
            assert!((0.0..360.0).contains(&position.ra_deg), "{planet:?}");
            assert!((-90.0..=90.0).contains(&position.dec_deg), "{planet:?}");
        }
        let moon = moon_equatorial(2_461_000.5);
        assert!((0.0..360.0).contains(&moon.ra_deg));
        assert!((-90.0..=90.0).contains(&moon.dec_deg));
    }

    #[test]
    fn unix_epoch_maps_to_julian_date() {
        assert!((julian_date(0.0) - 2_440_587.5).abs() < f64::EPSILON);
    }
}
