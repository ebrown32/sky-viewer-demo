mod astro;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use astro::{Equatorial, Horizontal, Planet};
use bevy::{
    color::LinearRgba,
    math::primitives::Sphere,
    prelude::*,
    render::{
        mesh::{Indices, Mesh},
        render_asset::RenderAssetUsages,
        render_resource::PrimitiveTopology,
    },
};

const SKY_RADIUS: f32 = 90.0;
const GALAXY_RADIUS: f32 = 98.0;
const BODY_RADIUS: f32 = 78.0;
const SKY_VERTICAL_FOV_DEGREES: f32 = 60.0;
const POLARIS: Equatorial = Equatorial {
    ra_deg: 37.954_560_67,
    dec_deg: 89.264_108_97,
};
const BETELGEUSE: Equatorial = Equatorial {
    ra_deg: 88.792_939,
    dec_deg: 7.407_064,
};

#[derive(Resource)]
struct Observer {
    latitude_deg: f64,
    longitude_deg: f64,
    last_refresh: f64,
    force_refresh: bool,
}

impl Default for Observer {
    fn default() -> Self {
        Self {
            latitude_deg: 47.6062,
            longitude_deg: -122.3321,
            last_refresh: 0.0,
            force_refresh: true,
        }
    }
}

#[derive(Resource)]
struct ViewDirection {
    azimuth_rad: f32,
    altitude_rad: f32,
    sensor_active: bool,
    manual_active: bool,
}

impl Default for ViewDirection {
    fn default() -> Self {
        Self {
            azimuth_rad: 0.0,
            altitude_rad: 25.0_f32.to_radians(),
            sensor_active: false,
            manual_active: false,
        }
    }
}

#[derive(Clone, Copy)]
struct Star {
    ra_hours: f64,
    dec_deg: f64,
    magnitude: f64,
    pm_ra: f64,
    pm_dec: f64,
}

#[derive(Resource)]
struct StarCatalog(Vec<Star>);

#[derive(Resource)]
struct SceneAssets {
    stars: Handle<Mesh>,
    galaxy: Handle<Mesh>,
}

#[derive(Clone, Copy, Component)]
struct CelestialBody(BodyKind);

#[derive(Clone, Copy, Component)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct ProjectedLabel {
    name: &'static str,
    color: Color,
}

#[derive(Clone, Copy, Component)]
struct SkyLabel(Equatorial);

#[derive(Clone, Copy)]
enum BodyKind {
    Sun,
    Moon,
    Planet(Planet),
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Observer::default())
        .insert_resource(ViewDirection::default())
        .insert_resource(StarCatalog(load_catalog()))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sky Viewer".to_owned(),
                canvas: Some("#sky-canvas".to_owned()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                read_browser_controls,
                pan_with_mouse,
                update_camera,
                update_sky,
                update_projected_labels,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    observer: Res<Observer>,
    catalog: Res<StarCatalog>,
) {
    commands.spawn(Camera3dBundle {
        projection: PerspectiveProjection {
            fov: SKY_VERTICAL_FOV_DEGREES.to_radians(),
            ..default()
        }
        .into(),
        ..default()
    });

    let now = unix_seconds();
    let julian_day = astro::julian_date(now);
    let sidereal_time =
        astro::local_sidereal_time_deg(julian_day, observer.longitude_deg);
    let star_mesh = meshes.add(build_star_mesh(
        &catalog.0,
        observer.latitude_deg,
        sidereal_time,
        julian_day,
    ));
    let star_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands
        .spawn(PbrBundle {
            mesh: star_mesh.clone(),
            material: star_material,
            ..default()
        })
        .insert(bevy::render::view::visibility::NoFrustumCulling);

    let galaxy_mesh = meshes.add(build_galaxy_mesh(
        observer.latitude_deg,
        sidereal_time,
    ));
    let galaxy_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });
    commands
        .spawn(PbrBundle {
            mesh: galaxy_mesh.clone(),
            material: galaxy_material,
            ..default()
        })
        .insert(bevy::render::view::visibility::NoFrustumCulling);
    commands.insert_resource(SceneAssets {
        stars: star_mesh,
        galaxy: galaxy_mesh,
    });

    let sphere_mesh = meshes.add(
        Sphere::new(0.5)
            .mesh()
            .ico(3)
            .expect("a low-subdivision sphere is valid"),
    );
    for kind in all_bodies() {
        let (name, color, scale) = body_appearance(kind);
        let material = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            unlit: true,
            ..default()
        });
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material,
                transform: Transform::from_scale(Vec3::splat(scale)),
                ..default()
            },
            CelestialBody(kind),
            ProjectedLabel { name, color },
        ));
    }

    for (name, position) in [("Polaris", POLARIS), ("Betelgeuse", BETELGEUSE)] {
        commands.spawn((
            SpatialBundle::default(),
            SkyLabel(position),
            ProjectedLabel {
                name,
                color: Color::WHITE,
            },
        ));
    }

    let milky_way_label = astro::galactic_to_equatorial(95.0, 4.0);
    commands.spawn((
        SpatialBundle::default(),
        SkyLabel(milky_way_label),
        ProjectedLabel {
            name: "Milky Way",
            color: Color::srgb(0.72, 0.74, 0.82),
        },
    ));
}

fn load_catalog() -> Vec<Star> {
    let stars = include_str!("../assets/stars.tsv")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split('\t').map(|value| {
                value
                    .parse::<f64>()
                    .expect("bundled HYG catalog fields must be numeric")
            });
            let star = Star {
                ra_hours: fields.next().expect("catalog rows have right ascension"),
                dec_deg: fields.next().expect("catalog rows have declination"),
                magnitude: fields.next().expect("catalog rows have magnitude"),
                pm_ra: fields.next().expect("catalog rows have proper motion in RA"),
                pm_dec: fields.next().expect("catalog rows have proper motion in declination"),
            };
            assert!(
                fields.next().is_none(),
                "bundled catalog rows must have exactly five fields"
            );
            star
        })
        .collect::<Vec<_>>();
    assert!(!stars.is_empty(), "the bundled HYG catalog must not be empty");
    stars
}

fn all_bodies() -> Vec<BodyKind> {
    let mut bodies = vec![BodyKind::Sun, BodyKind::Moon];
    bodies.extend(Planet::ALL.into_iter().map(BodyKind::Planet));
    bodies
}

fn body_appearance(kind: BodyKind) -> (&'static str, Color, f32) {
    match kind {
        BodyKind::Sun => ("Sun", Color::srgb(1.0, 0.78, 0.24), 2.0),
        BodyKind::Moon => ("Moon", Color::srgb(0.88, 0.91, 1.0), 0.8),
        BodyKind::Planet(Planet::Mercury) => {
            ("Mercury", Color::srgb(0.66, 0.65, 0.62), 0.44)
        }
        BodyKind::Planet(Planet::Venus) => {
            ("Venus", Color::srgb(1.0, 0.83, 0.61), 0.62)
        }
        BodyKind::Planet(Planet::Mars) => {
            ("Mars", Color::srgb(0.93, 0.36, 0.24), 0.52)
        }
        BodyKind::Planet(Planet::Jupiter) => {
            ("Jupiter", Color::srgb(0.91, 0.77, 0.59), 0.9)
        }
        BodyKind::Planet(Planet::Saturn) => {
            ("Saturn", Color::srgb(0.91, 0.79, 0.54), 0.8)
        }
        BodyKind::Planet(Planet::Uranus) => {
            ("Uranus", Color::srgb(0.5, 0.82, 0.87), 0.6)
        }
        BodyKind::Planet(Planet::Neptune) => {
            ("Neptune", Color::srgb(0.35, 0.54, 0.95), 0.6)
        }
    }
}

fn read_browser_controls(
    mut observer: ResMut<Observer>,
    mut view: ResMut<ViewDirection>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&mut observer, &mut view);

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Reflect;
        use wasm_bindgen::JsValue;

        if let Some(window) = web_sys::window() {
            if let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) {
                let latitude = number_property(&state, "latitude");
                let longitude = number_property(&state, "longitude");
                if let (Some(latitude), Some(longitude)) = (latitude, longitude) {
                    if (-90.0..=90.0).contains(&latitude)
                        && (-180.0..=180.0).contains(&longitude)
                        && ((latitude - observer.latitude_deg).abs() > 1.0e-5
                            || (longitude - observer.longitude_deg).abs() > 1.0e-5)
                    {
                        observer.latitude_deg = latitude;
                        observer.longitude_deg = longitude;
                        observer.force_refresh = true;
                    }
                }

                let sensor_active = bool_property(&state, "sensorActive").unwrap_or(false);
                let manual_active = bool_property(&state, "manualActive").unwrap_or(false);
                view.sensor_active = sensor_active;
                view.manual_active = manual_active;
                if sensor_active || manual_active {
                    if let (Some(heading), Some(altitude)) = (
                        number_property(&state, "heading"),
                        number_property(&state, "altitude"),
                    ) {
                        if heading.is_finite() && altitude.is_finite() {
                            view.azimuth_rad = (heading as f32).to_radians();
                            view.altitude_rad =
                                (altitude as f32).clamp(-85.0, 85.0).to_radians();
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn number_property(value: &wasm_bindgen::JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(value, &wasm_bindgen::JsValue::from_str(key))
        .ok()?
        .as_f64()
}

#[cfg(target_arch = "wasm32")]
fn bool_property(value: &wasm_bindgen::JsValue, key: &str) -> Option<bool> {
    js_sys::Reflect::get(value, &wasm_bindgen::JsValue::from_str(key))
        .ok()?
        .as_bool()
}

fn pan_with_mouse(
    mut motion: EventReader<bevy::input::mouse::MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut view: ResMut<ViewDirection>,
) {
    if view.sensor_active || view.manual_active {
        return;
    }
    for event in motion.read() {
        if buttons.pressed(MouseButton::Left) {
            view.azimuth_rad -= event.delta.x * 0.0035;
            view.altitude_rad = (view.altitude_rad - event.delta.y * 0.0035)
                .clamp(-85.0_f32.to_radians(), 85.0_f32.to_radians());
        }
    }
}

fn update_camera(view: Res<ViewDirection>, mut cameras: Query<&mut Transform, With<Camera3d>>) {
    let camera_rotation = Quat::from_rotation_y(std::f32::consts::PI + view.azimuth_rad)
        * Quat::from_rotation_x(view.altitude_rad);
    for mut transform in &mut cameras {
        transform.rotation = camera_rotation;
    }
}

fn update_projected_labels(
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    labels: Query<(&ProjectedLabel, &Transform, &Visibility)>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&cameras, &labels);

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Reflect;
        use wasm_bindgen::JsValue;

        let projected_labels = js_sys::Array::new();
        if let Ok((camera, transform)) = cameras.get_single() {
            let camera_transform = GlobalTransform::from(*transform);
            for (label, transform, visibility) in &labels {
                if *visibility != Visibility::Visible {
                    continue;
                }
                let Some(position) =
                    camera.world_to_viewport(&camera_transform, transform.translation)
                else {
                    continue;
                };
                let color = label.color.to_srgba();
                let projected_label = js_sys::Array::new();
                projected_label.push(&JsValue::from_str(label.name));
                projected_label.push(&JsValue::from_f64(position.x as f64));
                projected_label.push(&JsValue::from_f64(position.y as f64));
                projected_label.push(&JsValue::from_f64(color.red as f64));
                projected_label.push(&JsValue::from_f64(color.green as f64));
                projected_label.push(&JsValue::from_f64(color.blue as f64));
                projected_labels.push(projected_label.as_ref());
            }
        }

        if let Some(window) = web_sys::window() {
            if let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) {
                let _ = Reflect::set(
                    &state,
                    &JsValue::from_str("projectedLabels"),
                    projected_labels.as_ref(),
                );
            }
        }
    }
}

fn update_sky(
    mut observer: ResMut<Observer>,
    view: Res<ViewDirection>,
    catalog: Res<StarCatalog>,
    scene: Res<SceneAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: Query<(&CelestialBody, &mut Transform, &mut Visibility)>,
    mut sky_labels: Query<
        (&SkyLabel, &mut Transform, &mut Visibility),
        Without<CelestialBody>,
    >,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &view;

    let now = unix_seconds();
    let julian_day = astro::julian_date(now);
    let sidereal_time =
        astro::local_sidereal_time_deg(julian_day, observer.longitude_deg);

    #[cfg(target_arch = "wasm32")]
    update_coordinate_readout(&view, observer.latitude_deg, sidereal_time);

    if !observer.force_refresh && now - observer.last_refresh < 30.0 {
        return;
    }

    *meshes
        .get_mut(&scene.stars)
        .expect("the star mesh must remain in the asset store") = build_star_mesh(
        &catalog.0,
        observer.latitude_deg,
        sidereal_time,
        julian_day,
    );
    *meshes
        .get_mut(&scene.galaxy)
        .expect("the Milky Way mesh must remain in the asset store") =
        build_galaxy_mesh(observer.latitude_deg, sidereal_time);

    for (body, mut transform, mut visibility) in &mut bodies {
        let position = body_equatorial(body.0, julian_day);
        let horizontal = astro::equatorial_to_horizontal(
            position,
            observer.latitude_deg,
            sidereal_time,
        );
        *visibility = if horizontal.altitude_deg > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.translation = horizontal_vector(horizontal) * BODY_RADIUS;
    }

    for (label, mut transform, mut visibility) in &mut sky_labels {
        let horizontal = astro::equatorial_to_horizontal(
            label.0,
            observer.latitude_deg,
            sidereal_time,
        );
        *visibility = if horizontal.altitude_deg > 0.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        set_label_transform(&mut transform, horizontal, SKY_RADIUS - 1.5);
    }

    observer.last_refresh = now;
    observer.force_refresh = false;
}

fn body_equatorial(kind: BodyKind, julian_day: f64) -> Equatorial {
    match kind {
        BodyKind::Sun => astro::sun_equatorial(julian_day),
        BodyKind::Moon => astro::moon_equatorial(julian_day),
        BodyKind::Planet(planet) => astro::planet_equatorial(planet, julian_day),
    }
}

fn build_star_mesh(
    stars: &[Star],
    latitude_deg: f64,
    sidereal_time_deg: f64,
    julian_day: f64,
) -> Mesh {
    let years_since_j2000 = (julian_day - 2_451_545.0) / 365.25;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for star in stars {
        let position = astro::proper_motion(
            star.ra_hours,
            star.dec_deg,
            star.pm_ra,
            star.pm_dec,
            years_since_j2000,
        );
        let horizontal =
            astro::equatorial_to_horizontal(position, latitude_deg, sidereal_time_deg);
        if horizontal.altitude_deg <= 0.0 {
            continue;
        }
        let direction = horizontal_vector(horizontal);
        let radius = 0.055 * 10.0_f32.powf(((5.0 - star.magnitude) as f32) * 0.16);
        append_quad(
            &mut positions,
            &mut normals,
            &mut colors,
            &mut indices,
            direction * SKY_RADIUS,
            direction,
            radius,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    create_mesh(positions, normals, colors, indices)
}

fn build_galaxy_mesh(latitude_deg: f64, sidereal_time_deg: f64) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let segments = 720;

    for index in 0..segments {
        let longitude_a = index as f64 * 360.0 / segments as f64;
        let longitude_b = (index + 1) as f64 * 360.0 / segments as f64;
        let points = [
            astro::galactic_to_equatorial(longitude_a, -7.0),
            astro::galactic_to_equatorial(longitude_b, -7.0),
            astro::galactic_to_equatorial(longitude_b, 7.0),
            astro::galactic_to_equatorial(longitude_a, 7.0),
        ];
        let horizontal = points.map(|point| {
            astro::equatorial_to_horizontal(point, latitude_deg, sidereal_time_deg)
        });
        if horizontal.iter().any(|point| point.altitude_deg <= 0.0) {
            continue;
        }

        let first_index = positions.len() as u32;
        for (corner, point) in horizontal.into_iter().enumerate() {
            let direction = horizontal_vector(point);
            positions.push((direction * GALAXY_RADIUS).to_array());
            normals.push(direction.to_array());
            let alpha = if corner == 0 || corner == 3 { 0.08 } else { 0.19 };
            colors.push([0.48, 0.50, 0.58, alpha]);
        }
        indices.extend_from_slice(&[
            first_index,
            first_index + 1,
            first_index + 2,
            first_index,
            first_index + 2,
            first_index + 3,
        ]);
    }
    create_mesh(positions, normals, colors, indices)
}

fn append_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    direction: Vec3,
    half_size: f32,
    color: [f32; 4],
) {
    let up_axis = if direction.y.abs() > 0.92 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let right = direction.cross(up_axis).normalize();
    let up = right.cross(direction).normalize();
    let first_index = positions.len() as u32;
    for vertex in [
        center - right * half_size - up * half_size,
        center + right * half_size - up * half_size,
        center + right * half_size + up * half_size,
        center - right * half_size + up * half_size,
    ] {
        positions.push(vertex.to_array());
        normals.push(direction.to_array());
        colors.push(color);
    }
    indices.extend_from_slice(&[
        first_index,
        first_index + 1,
        first_index + 2,
        first_index,
        first_index + 2,
        first_index + 3,
    ]);
}

fn create_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn horizontal_vector(horizontal: Horizontal) -> Vec3 {
    let azimuth = (horizontal.azimuth_deg as f32).to_radians();
    let altitude = (horizontal.altitude_deg as f32).to_radians();
    Vec3::new(
        altitude.cos() * azimuth.sin(),
        altitude.sin(),
        altitude.cos() * azimuth.cos(),
    )
}

fn set_label_transform(transform: &mut Transform, horizontal: Horizontal, radius: f32) {
    let direction = horizontal_vector(horizontal);
    transform.translation = direction * radius;
    transform.look_at(direction * (radius + 1.0), Vec3::Y);
}

fn unix_seconds() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        return js_sys::Date::now() / 1_000.0;
    }

    #[cfg(not(target_arch = "wasm32"))]
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |duration| duration.as_secs_f64())
}

#[cfg(target_arch = "wasm32")]
fn update_coordinate_readout(view: &ViewDirection, latitude_deg: f64, sidereal_time_deg: f64) {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let horizontal = Horizontal {
        azimuth_deg: (view.azimuth_rad.to_degrees() as f64).rem_euclid(360.0),
        altitude_deg: view.altitude_rad.to_degrees() as f64,
    };
    let equatorial =
        astro::horizontal_to_equatorial(horizontal, latitude_deg, sidereal_time_deg);
    if let Some(window) = web_sys::window() {
        if let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) {
            let _ = Reflect::set(
                &state,
                &JsValue::from_str("rightAscension"),
                &JsValue::from_f64(equatorial.ra_deg),
            );
            let _ = Reflect::set(
                &state,
                &JsValue::from_str("declination"),
                &JsValue::from_f64(equatorial.dec_deg),
            );
        }
    }
}
