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
const STAR_BATCH_SIZE: usize = 32;
const GALAXY_BATCH_SIZE: usize = 24;
const GALAXY_SEGMENTS: usize = 720;
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
    force_refresh: bool,
    #[cfg(target_arch = "wasm32")]
    refresh_reason: Option<SkyMeshRefreshReason>,
}

impl Default for Observer {
    fn default() -> Self {
        Self {
            latitude_deg: 47.6062,
            longitude_deg: -122.3321,
            force_refresh: true,
            #[cfg(target_arch = "wasm32")]
            refresh_reason: Some(SkyMeshRefreshReason::Startup),
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
enum SkyMeshRefreshReason {
    Startup,
    LocationChanged {
        previous_latitude_deg: f64,
        previous_longitude_deg: f64,
    },
    Forced,
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

#[derive(Default, Resource)]
struct PendingSkyMeshBuild(Option<SkyMeshBuild>);

struct SkyMeshBuild {
    latitude_deg: f64,
    sidereal_time_deg: f64,
    julian_day: f64,
    next_star: usize,
    next_galaxy_segment: usize,
    stars: MeshBuffers,
    galaxy: MeshBuffers,
}

impl SkyMeshBuild {
    fn new(latitude_deg: f64, sidereal_time_deg: f64, julian_day: f64) -> Self {
        Self {
            latitude_deg,
            sidereal_time_deg,
            julian_day,
            next_star: 0,
            next_galaxy_segment: 0,
            stars: MeshBuffers::default(),
            galaxy: MeshBuffers::default(),
        }
    }

    fn build_chunk(&mut self, stars: &[Star]) -> bool {
        let star_end = (self.next_star + STAR_BATCH_SIZE).min(stars.len());
        append_star_mesh_range(
            &mut self.stars,
            stars,
            self.next_star,
            star_end,
            self.latitude_deg,
            self.sidereal_time_deg,
            self.julian_day,
        );
        self.next_star = star_end;

        let galaxy_end =
            (self.next_galaxy_segment + GALAXY_BATCH_SIZE).min(GALAXY_SEGMENTS);
        append_galaxy_mesh_range(
            &mut self.galaxy,
            self.next_galaxy_segment,
            galaxy_end,
            self.latitude_deg,
            self.sidereal_time_deg,
        );
        self.next_galaxy_segment = galaxy_end;

        self.next_star == stars.len() && self.next_galaxy_segment == GALAXY_SEGMENTS
    }
}

#[derive(Default)]
struct MeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuffers {
    fn into_mesh(self) -> Mesh {
        create_mesh(self.positions, self.normals, self.colors, self.indices)
    }
}

#[derive(Clone, Copy, Component)]
struct CelestialBody(BodyKind);

#[derive(Component)]
struct SunDirectionLight;

#[derive(Clone, Copy, Component)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct ProjectedLabel {
    name: &'static str,
    color: Color,
    category: LabelCategory,
    magnitude: Option<f64>,
}

#[derive(Clone, Copy, Component)]
struct SkyLabel(Equatorial);

#[derive(Component)]
struct PipelineWarmup;

#[derive(Clone, Copy)]
enum LabelCategory {
    SunMoon,
    Planet,
    Constellation,
    Star,
    SagittariusA,
    Galaxy,
}

impl LabelCategory {
    #[cfg(target_arch = "wasm32")]
    fn as_str(self) -> &'static str {
        match self {
            Self::SunMoon => "sun-moon",
            Self::Planet => "planets",
            Self::Constellation => "constellations",
            Self::Star => "star",
            Self::SagittariusA => "sagittarius-a",
            Self::Galaxy => "galaxies",
        }
    }
}

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
        .insert_resource(PendingSkyMeshBuild::default())
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
                .chain()
                .run_if(browser_startup_ready),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    observer: Res<Observer>,
) {
    commands.spawn(Camera3dBundle {
        camera: Camera {
            is_active: false,
            ..default()
        },
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
    let sun_horizontal = astro::equatorial_to_horizontal(
        astro::sun_equatorial(julian_day),
        observer.latitude_deg,
        sidereal_time,
    );
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 100_000.0,
                shadows_enabled: false,
                ..default()
            },
            transform: sun_light_transform(horizontal_vector(sun_horizontal)),
            ..default()
        },
        SunDirectionLight,
    ));
    // Non-empty placeholders queue these sky pipeline variants while startup is still covered.
    let star_mesh = meshes.add(pipeline_warmup_mesh());
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

    let galaxy_mesh = meshes.add(pipeline_warmup_mesh());
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
    let mut warmed_unlit_body_pipeline = false;
    for kind in all_bodies() {
        let (name, color, scale) = body_appearance(kind);
        let moon = matches!(kind, BodyKind::Moon);
        let material = materials.add(StandardMaterial {
            base_color: color,
            emissive: if moon {
                LinearRgba::BLACK
            } else {
                color.into()
            },
            unlit: !moon,
            ..default()
        });
        if moon || !warmed_unlit_body_pipeline {
            // Warm the sphere variants even when the corresponding body is below the horizon.
            commands.spawn((
                PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_translation(Vec3::splat(100_000.0)),
                    ..default()
                },
                bevy::render::view::visibility::NoFrustumCulling,
                PipelineWarmup,
            ));
            if !moon {
                warmed_unlit_body_pipeline = true;
            }
        }
        commands.spawn((
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material,
                transform: Transform::from_scale(Vec3::splat(scale)),
                ..default()
            },
            CelestialBody(kind),
            ProjectedLabel {
                name,
                color,
                category: match kind {
                    BodyKind::Sun | BodyKind::Moon => LabelCategory::SunMoon,
                    BodyKind::Planet(_) => LabelCategory::Planet,
                },
                magnitude: None,
            },
        ));
    }

    let named_stars = [
        ("Polaris", POLARIS, 1.98),
        ("Betelgeuse", BETELGEUSE, 0.42),
        (
            "Sirius",
            Equatorial {
                ra_deg: 101.287,
                dec_deg: -16.716,
            },
            -1.46,
        ),
        (
            "Canopus",
            Equatorial {
                ra_deg: 95.988,
                dec_deg: -52.696,
            },
            -0.74,
        ),
        (
            "Rigil Kentaurus",
            Equatorial {
                ra_deg: 219.902,
                dec_deg: -60.834,
            },
            -0.27,
        ),
        (
            "Arcturus",
            Equatorial {
                ra_deg: 213.915,
                dec_deg: 19.182,
            },
            -0.05,
        ),
        (
            "Vega",
            Equatorial {
                ra_deg: 279.235,
                dec_deg: 38.784,
            },
            0.03,
        ),
        (
            "Capella",
            Equatorial {
                ra_deg: 79.172,
                dec_deg: 45.998,
            },
            0.08,
        ),
        (
            "Rigel",
            Equatorial {
                ra_deg: 78.634,
                dec_deg: -8.202,
            },
            0.13,
        ),
        (
            "Procyon",
            Equatorial {
                ra_deg: 114.825,
                dec_deg: 5.225,
            },
            0.34,
        ),
        (
            "Achernar",
            Equatorial {
                ra_deg: 24.429,
                dec_deg: -57.237,
            },
            0.46,
        ),
        (
            "Hadar",
            Equatorial {
                ra_deg: 210.956,
                dec_deg: -60.373,
            },
            0.61,
        ),
        (
            "Altair",
            Equatorial {
                ra_deg: 297.696,
                dec_deg: 8.868,
            },
            0.77,
        ),
        (
            "Acrux",
            Equatorial {
                ra_deg: 186.65,
                dec_deg: -63.10,
            },
            0.76,
        ),
        (
            "Aldebaran",
            Equatorial {
                ra_deg: 68.98,
                dec_deg: 16.51,
            },
            0.85,
        ),
        (
            "Spica",
            Equatorial {
                ra_deg: 201.298,
                dec_deg: -11.16,
            },
            0.98,
        ),
        (
            "Antares",
            Equatorial {
                ra_deg: 247.35,
                dec_deg: -26.43,
            },
            1.06,
        ),
        (
            "Pollux",
            Equatorial {
                ra_deg: 116.329,
                dec_deg: 28.026,
            },
            1.14,
        ),
        (
            "Fomalhaut",
            Equatorial {
                ra_deg: 344.412,
                dec_deg: -29.622,
            },
            1.16,
        ),
        (
            "Deneb",
            Equatorial {
                ra_deg: 310.358,
                dec_deg: 45.280,
            },
            1.25,
        ),
        (
            "Regulus",
            Equatorial {
                ra_deg: 152.093,
                dec_deg: 11.967,
            },
            1.35,
        ),
        (
            "Castor",
            Equatorial {
                ra_deg: 113.650,
                dec_deg: 31.888,
            },
            1.58,
        ),
    ];
    for (name, position, magnitude) in named_stars {
        commands.spawn((
            SpatialBundle::default(),
            SkyLabel(position),
            ProjectedLabel {
                name,
                color: Color::WHITE,
                category: LabelCategory::Star,
                magnitude: Some(magnitude),
            },
        ));
    }

    let constellations = [
        ("Andromeda", 11.25, 37.0),
        ("Aquarius", 335.0, -10.0),
        ("Cassiopeia", 15.0, 60.0),
        ("Cygnus", 308.75, 42.0),
        ("Gemini", 105.0, 22.0),
        ("Leo", 157.5, 15.0),
        ("Lyra", 282.5, 36.0),
        ("Orion", 83.75, -1.0),
        ("Pegasus", 337.5, 20.0),
        ("Scorpius", 247.5, -30.0),
        ("Taurus", 67.5, 18.0),
        ("Ursa Major", 165.0, 55.0),
        ("Ursa Minor", 225.0, 75.0),
    ];
    for (name, ra_deg, dec_deg) in constellations {
        commands.spawn((
            SpatialBundle::default(),
            SkyLabel(Equatorial { ra_deg, dec_deg }),
            ProjectedLabel {
                name,
                color: Color::srgb(0.72, 0.74, 0.82),
                category: LabelCategory::Constellation,
                magnitude: None,
            },
        ));
    }

    let sagittarius_a = astro::galactic_to_equatorial(0.0, 0.0);
    commands.spawn((
        SpatialBundle::default(),
        SkyLabel(sagittarius_a),
        ProjectedLabel {
            name: "Sagittarius A*",
            color: Color::srgb(0.94, 0.68, 0.36),
            category: LabelCategory::SagittariusA,
            magnitude: None,
        },
    ));

    let milky_way_label = astro::galactic_to_equatorial(95.0, 4.0);
    commands.spawn((
        SpatialBundle::default(),
        SkyLabel(milky_way_label),
        ProjectedLabel {
            name: "Milky Way",
            color: Color::srgb(0.72, 0.74, 0.82),
            category: LabelCategory::Galaxy,
            magnitude: None,
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
                        observer.refresh_reason = Some(SkyMeshRefreshReason::LocationChanged {
                            previous_latitude_deg: observer.latitude_deg,
                            previous_longitude_deg: observer.longitude_deg,
                        });
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
                                (altitude as f32).clamp(-90.0, 90.0).to_radians();
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

fn browser_startup_ready() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Reflect;
        use wasm_bindgen::JsValue;

        let Some(window) = web_sys::window() else {
            return false;
        };
        let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) else {
            return false;
        };
        return bool_property(&state, "startupReady").unwrap_or(false);
    }

    #[cfg(not(target_arch = "wasm32"))]
    true
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

fn update_camera(
    view: Res<ViewDirection>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<Camera3d>>,
) {
    let camera_rotation = view_rotation(&view);
    for (mut camera, mut transform) in &mut cameras {
        camera.is_active = true;
        transform.rotation = camera_rotation;
    }
}

fn view_rotation(view: &ViewDirection) -> Quat {
    Quat::from_rotation_y(-view.azimuth_rad)
        * Quat::from_rotation_x(view.altitude_rad)
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

        let projection_started = js_sys::Date::now();
        let projected_labels = js_sys::Array::new();
        let objects = js_sys::Array::new();
        let mut label_count = 0;
        if let Ok((camera, transform)) = cameras.get_single() {
            let camera_transform = GlobalTransform::from(*transform);
            for (label, transform, visibility) in &labels {
                label_count += 1;
                let direction = transform.translation.normalize();
                let heading = (direction.x.atan2(-direction.z).to_degrees() as f64)
                    .rem_euclid(360.0);
                let altitude = direction.y.clamp(-1.0, 1.0).asin().to_degrees() as f64;
                let object = js_sys::Array::new();
                object.push(&JsValue::from_str(label.name));
                object.push(&JsValue::from_f64(heading));
                object.push(&JsValue::from_f64(altitude));
                objects.push(object.as_ref());

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
                projected_label.push(&JsValue::from_str(label.category.as_str()));
                projected_label.push(&JsValue::from_f64(label.magnitude.unwrap_or(f64::NAN)));
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
                let _ = Reflect::set(&state, &JsValue::from_str("objects"), objects.as_ref());
            }
        }
        let projection_duration = js_sys::Date::now() - projection_started;
        if projection_duration >= 100.0 {
            log_slow_operation(
                &format!("projected label update ({label_count} labels)"),
                projection_duration,
            );
        }
    }
}

fn update_sky(
    mut commands: Commands,
    mut observer: ResMut<Observer>,
    mut pending_build: ResMut<PendingSkyMeshBuild>,
    view: Res<ViewDirection>,
    catalog: Res<StarCatalog>,
    scene: Res<SceneAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut bodies: Query<(&CelestialBody, &mut Transform, &mut Visibility)>,
    mut sun_lights: Query<
        &mut Transform,
        (
            With<SunDirectionLight>,
            Without<CelestialBody>,
            Without<SkyLabel>,
        ),
    >,
    mut sky_labels: Query<
        (&SkyLabel, &ProjectedLabel, &mut Transform, &mut Visibility),
        Without<CelestialBody>,
    >,
    pipeline_warmups: Query<Entity, With<PipelineWarmup>>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &view;

    #[cfg(target_arch = "wasm32")]
    let update_started = js_sys::Date::now();
    let now = unix_seconds();
    let julian_day = astro::julian_date(now);
    let sidereal_time =
        astro::local_sidereal_time_deg(julian_day, observer.longitude_deg);
    let sun_horizontal = astro::equatorial_to_horizontal(
        astro::sun_equatorial(julian_day),
        observer.latitude_deg,
        sidereal_time,
    );
    let sun_direction = horizontal_vector(sun_horizontal);
    for mut light_transform in &mut sun_lights {
        *light_transform = sun_light_transform(sun_direction);
    }

    #[cfg(target_arch = "wasm32")]
    update_coordinate_readout(&view, observer.latitude_deg, sidereal_time);

    #[cfg(target_arch = "wasm32")]
    let positions_started = js_sys::Date::now();
    for (body, mut transform, mut visibility) in &mut bodies {
        let position = body_equatorial(body.0, julian_day);
        let horizontal = astro::equatorial_to_horizontal(
            position,
            observer.latitude_deg,
            sidereal_time,
        );
        *visibility = Visibility::Visible;
        transform.translation = horizontal_vector(horizontal) * BODY_RADIUS;
    }

    for (label, _, mut transform, mut visibility) in &mut sky_labels {
        let horizontal = astro::equatorial_to_horizontal(
            label.0,
            observer.latitude_deg,
            sidereal_time,
        );
        *visibility = Visibility::Visible;
        set_label_transform(&mut transform, horizontal, SKY_RADIUS - 1.5);
    }
    #[cfg(target_arch = "wasm32")]
    log_slow_operation(
        "celestial object positioning",
        js_sys::Date::now() - positions_started,
    );

    if observer.force_refresh {
        #[cfg(target_arch = "wasm32")]
        log_sky_mesh_refresh(
            observer
                .refresh_reason
                .unwrap_or(SkyMeshRefreshReason::Forced),
            observer.latitude_deg,
            observer.longitude_deg,
            catalog.0.len(),
        );
        #[cfg(target_arch = "wasm32")]
        {
            observer.refresh_reason = None;
        }
        pending_build.0 = Some(SkyMeshBuild::new(
            observer.latitude_deg,
            sidereal_time,
            julian_day,
        ));
        observer.force_refresh = false;
    }

    if let Some(build) = &mut pending_build.0 {
        #[cfg(target_arch = "wasm32")]
        let chunk_started = js_sys::Date::now();
        let build_complete = build.build_chunk(&catalog.0);
        #[cfg(target_arch = "wasm32")]
        log_slow_operation("mesh build chunk", js_sys::Date::now() - chunk_started);
        if build_complete {
            let build = pending_build
                .0
                .take()
                .expect("a completed sky mesh build must be pending");
            #[cfg(target_arch = "wasm32")]
            let finalize_started = js_sys::Date::now();
            *meshes
                .get_mut(&scene.stars)
                .expect("the star mesh must remain in the asset store") = build.stars.into_mesh();
            *meshes
                .get_mut(&scene.galaxy)
                .expect("the Milky Way mesh must remain in the asset store") =
                build.galaxy.into_mesh();
            for entity in &pipeline_warmups {
                commands.entity(entity).despawn();
            }
            #[cfg(target_arch = "wasm32")]
            log_slow_operation(
                "mesh finalize and upload",
                js_sys::Date::now() - finalize_started,
            );
            #[cfg(target_arch = "wasm32")]
            mark_browser_sky_ready();
        }
    }
    #[cfg(target_arch = "wasm32")]
    log_slow_operation("sky update system", js_sys::Date::now() - update_started);
}

#[cfg(target_arch = "wasm32")]
fn mark_browser_sky_ready() {
    use js_sys::Reflect;
    use wasm_bindgen::JsValue;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) else {
        return;
    };
    let _ = Reflect::set(
        &state,
        &JsValue::from_str("skyReady"),
        &JsValue::from_bool(true),
    );
}

#[cfg(target_arch = "wasm32")]
fn log_sky_mesh_refresh(
    reason: SkyMeshRefreshReason,
    latitude_deg: f64,
    longitude_deg: f64,
    star_count: usize,
) {
    use js_sys::Reflect;
    use wasm_bindgen::{JsCast, JsValue};

    let reason_details = match reason {
        SkyMeshRefreshReason::LocationChanged {
            previous_latitude_deg,
            previous_longitude_deg,
        } => format!(
            "reason=location changed, from=({previous_latitude_deg:.5}, \
             {previous_longitude_deg:.5}),"
        ),
        SkyMeshRefreshReason::Startup => "reason=startup,".to_owned(),
        SkyMeshRefreshReason::Forced => "reason=forced,".to_owned(),
    };
    let message = JsValue::from_str(&format!(
        "[Sky Viewer] Starting sky mesh rebuild: {reason_details} \
         to=({latitude_deg:.5}, {longitude_deg:.5}), stars={star_count}, \
         galaxySegments={GALAXY_SEGMENTS}"
    ));

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(console) = Reflect::get(window.as_ref(), &JsValue::from_str("console")) else {
        return;
    };
    let Ok(info) = Reflect::get(&console, &JsValue::from_str("info")) else {
        return;
    };
    let Some(info) = info.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let _ = info.call1(&console, &message);
}

#[cfg(target_arch = "wasm32")]
fn log_slow_operation(stage: &str, duration_ms: f64) {
    if duration_ms < 100.0 {
        return;
    }

    use js_sys::Reflect;
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(console) = Reflect::get(window.as_ref(), &JsValue::from_str("console")) else {
        return;
    };
    let Ok(warn) = Reflect::get(&console, &JsValue::from_str("warn")) else {
        return;
    };
    let Some(warn) = warn.dyn_ref::<js_sys::Function>() else {
        return;
    };
    let message = JsValue::from_str(&format!(
        "[Sky Viewer] Slow {stage}: {duration_ms:.1} ms"
    ));
    let _ = warn.call1(&console, &message);
}

fn body_equatorial(kind: BodyKind, julian_day: f64) -> Equatorial {
    match kind {
        BodyKind::Sun => astro::sun_equatorial(julian_day),
        BodyKind::Moon => astro::moon_equatorial(julian_day),
        BodyKind::Planet(planet) => astro::planet_equatorial(planet, julian_day),
    }
}

fn append_star_mesh_range(
    buffers: &mut MeshBuffers,
    stars: &[Star],
    start: usize,
    end: usize,
    latitude_deg: f64,
    sidereal_time_deg: f64,
    julian_day: f64,
) {
    let years_since_j2000 = (julian_day - 2_451_545.0) / 365.25;
    for star in &stars[start..end] {
        let position = astro::proper_motion(
            star.ra_hours,
            star.dec_deg,
            star.pm_ra,
            star.pm_dec,
            years_since_j2000,
        );
        let horizontal =
            astro::equatorial_to_horizontal(position, latitude_deg, sidereal_time_deg);
        let direction = horizontal_vector(horizontal);
        let radius = 0.055 * 10.0_f32.powf(((5.0 - star.magnitude) as f32) * 0.16);
        let color = if horizontal.altitude_deg < 0.0 {
            [0.48, 0.54, 0.68, 0.35]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        append_quad(
            buffers,
            direction * SKY_RADIUS,
            direction,
            radius,
            color,
        );
    }
}

fn append_galaxy_mesh_range(
    buffers: &mut MeshBuffers,
    start: usize,
    end: usize,
    latitude_deg: f64,
    sidereal_time_deg: f64,
) {
    for index in start..end {
        let longitude_a = index as f64 * 360.0 / GALAXY_SEGMENTS as f64;
        let longitude_b = (index + 1) as f64 * 360.0 / GALAXY_SEGMENTS as f64;
        let points = [
            astro::galactic_to_equatorial(longitude_a, -7.0),
            astro::galactic_to_equatorial(longitude_b, -7.0),
            astro::galactic_to_equatorial(longitude_b, 7.0),
            astro::galactic_to_equatorial(longitude_a, 7.0),
        ];
        let horizontal = points.map(|point| {
            astro::equatorial_to_horizontal(point, latitude_deg, sidereal_time_deg)
        });

        let first_index = buffers.positions.len() as u32;
        for (corner, point) in horizontal.into_iter().enumerate() {
            let direction = horizontal_vector(point);
            buffers
                .positions
                .push((direction * GALAXY_RADIUS).to_array());
            buffers.normals.push(direction.to_array());
            let base_alpha = if corner == 0 || corner == 3 { 0.08 } else { 0.19 };
            let alpha = if point.altitude_deg < 0.0 {
                base_alpha * 0.35
            } else {
                base_alpha
            };
            buffers.colors.push([0.48, 0.50, 0.58, alpha]);
        }
        buffers.indices.extend_from_slice(&[
            first_index,
            first_index + 1,
            first_index + 2,
            first_index,
            first_index + 2,
            first_index + 3,
        ]);
    }
}

fn pipeline_warmup_mesh() -> Mesh {
    let mut buffers = MeshBuffers::default();
    append_quad(
        &mut buffers,
        Vec3::ZERO,
        Vec3::Z,
        0.001,
        [0.0, 0.0, 0.0, 0.0],
    );
    buffers.into_mesh()
}

fn append_quad(
    buffers: &mut MeshBuffers,
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
    let first_index = buffers.positions.len() as u32;
    for vertex in [
        center - right * half_size - up * half_size,
        center + right * half_size - up * half_size,
        center + right * half_size + up * half_size,
        center - right * half_size + up * half_size,
    ] {
        buffers.positions.push(vertex.to_array());
        buffers.normals.push(direction.to_array());
        buffers.colors.push(color);
    }
    buffers.indices.extend_from_slice(&[
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
        -altitude.cos() * azimuth.cos(),
    )
}

fn sun_light_transform(sun_direction: Vec3) -> Transform {
    let light_position = sun_direction * 100.0;
    let up = if sun_direction.y.abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    Transform::from_translation(light_position).looking_at(Vec3::ZERO, up)
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
    let heading = (view.azimuth_rad.to_degrees() as f64).rem_euclid(360.0);
    let altitude = view.altitude_rad.to_degrees() as f64;
    if let Some(window) = web_sys::window() {
        if let Ok(state) = Reflect::get(window.as_ref(), &JsValue::from_str("skyState")) {
            let _ = Reflect::set(
                &state,
                &JsValue::from_str("heading"),
                &JsValue::from_f64(heading),
            );
            let _ = Reflect::set(
                &state,
                &JsValue::from_str("altitude"),
                &JsValue::from_f64(altitude),
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::mesh::VertexAttributeValues;

    #[test]
    fn pitching_changes_elevation_without_changing_compass_heading() {
        let azimuth = 123.0_f32.to_radians();
        for altitude in [-60.0_f32, 0.0, 60.0] {
            let view = ViewDirection {
                azimuth_rad: azimuth,
                altitude_rad: altitude.to_radians(),
                sensor_active: false,
                manual_active: false,
            };
            let rotation = view_rotation(&view);
            let forward = rotation * Vec3::NEG_Z;
            let expected = Vec3::new(
                altitude.to_radians().cos() * azimuth.sin(),
                altitude.to_radians().sin(),
                -altitude.to_radians().cos() * azimuth.cos(),
            );
            assert!((forward - expected).length() < 1.0e-5);
            let right = rotation * Vec3::X;
            let expected_right = Vec3::new(azimuth.cos(), 0.0, azimuth.sin());
            assert!((right - expected_right).length() < 1.0e-5);
        }
    }

    #[test]
    fn camera_yaw_and_sky_vectors_use_an_unmirrored_local_horizon_frame() {
        for heading in [0.0_f32, 45.0, 90.0, 180.0, 270.0, 359.0] {
            for altitude in [-90.0_f32, -60.0, 0.0, 60.0, 90.0] {
                let view = ViewDirection {
                    azimuth_rad: heading.to_radians(),
                    altitude_rad: altitude.to_radians(),
                    ..default()
                };
                let rotation = view_rotation(&view);
                let forward = horizontal_vector(Horizontal {
                    azimuth_deg: heading as f64,
                    altitude_deg: altitude as f64,
                });
                assert!((rotation * Vec3::NEG_Z - forward).length() < 1.0e-5);
                let east_of_view = horizontal_vector(Horizontal {
                    azimuth_deg: (heading + 10.0) as f64,
                    altitude_deg: 0.0,
                });
                assert!((rotation.inverse() * east_of_view).x > 0.0);
                let zenith_in_view = rotation.inverse() * Vec3::Y;
                assert!(zenith_in_view.x.abs() < 1.0e-5);
                assert!(zenith_in_view.y >= -1.0e-5);
            }
        }
        assert!((horizontal_vector(Horizontal {
            azimuth_deg: 0.0, altitude_deg: 0.0,
        }) - Vec3::NEG_Z).length() < 1.0e-5);
        assert!((horizontal_vector(Horizontal {
            azimuth_deg: 90.0, altitude_deg: 0.0,
        }) - Vec3::X).length() < 1.0e-5);
    }

    #[test]
    fn sunlight_points_from_the_sun_toward_the_sky_scene() {
        for sun_direction in [
            Vec3::new(0.3, -0.4, 0.5).normalize(),
            Vec3::Y,
            Vec3::NEG_Y,
        ] {
            let transform = sun_light_transform(sun_direction);
            let ray_direction = transform.rotation * Vec3::NEG_Z;
            assert!((ray_direction + sun_direction).length() < 1.0e-5);
        }
    }

    #[test]
    fn update_sky_system_initializes_without_query_conflicts() {
        let mut system = bevy::ecs::system::IntoSystem::into_system(update_sky);
        system.initialize(&mut World::new());
    }

    #[test]
    fn star_mesh_includes_below_horizon_stars_with_dimmed_colors() {
        let stars = [
            Star {
                ra_hours: 0.0,
                dec_deg: 0.0,
                magnitude: 1.0,
                pm_ra: 0.0,
                pm_dec: 0.0,
            },
            Star {
                ra_hours: 12.0,
                dec_deg: 0.0,
                magnitude: 1.0,
                pm_ra: 0.0,
                pm_dec: 0.0,
            },
        ];
        let mut buffers = MeshBuffers::default();

        append_star_mesh_range(&mut buffers, &stars, 0, stars.len(), 0.0, 0.0, 2_451_545.0);

        assert_eq!(buffers.indices.len(), 12);
        assert_eq!(buffers.colors[0][3], 1.0);
        assert!(buffers.colors[4][3] < buffers.colors[0][3]);
    }

    #[test]
    fn milky_way_mesh_spans_the_full_sphere_and_dims_below_horizon() {
        let mut buffers = MeshBuffers::default();

        append_galaxy_mesh_range(&mut buffers, 0, GALAXY_SEGMENTS, 0.0, 0.0);

        assert_eq!(buffers.indices.len(), GALAXY_SEGMENTS * 6);
        assert!(buffers.colors.iter().any(|color| color[3] < 0.08));
        assert!(buffers.colors.iter().any(|color| color[3] >= 0.08));
    }

    #[test]
    fn pipeline_warmup_mesh_is_nonempty_and_transparent() {
        let mesh = pipeline_warmup_mesh();
        assert_eq!(mesh.count_vertices(), 4);
        let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap();
        let VertexAttributeValues::Float32x4(colors) = colors else {
            panic!("pipeline warmup colors must use the mesh color format");
        };
        assert!(colors.iter().all(|color| color[3] == 0.0));
    }

    #[test]
    fn batched_sky_mesh_build_matches_full_mesh_data() {
        let stars = vec![
            Star {
                ra_hours: 2.5,
                dec_deg: 89.2,
                magnitude: 1.0,
                pm_ra: 0.0,
                pm_dec: 0.0,
            };
            STAR_BATCH_SIZE + 1
        ];
        let latitude_deg = 47.6062;
        let sidereal_time_deg = 123.4;
        let julian_day = 2_461_000.5;
        let mut build = SkyMeshBuild::new(latitude_deg, sidereal_time_deg, julian_day);

        for chunk in 0..GALAXY_SEGMENTS / GALAXY_BATCH_SIZE {
            assert_eq!(
                build.build_chunk(&stars),
                chunk + 1 == GALAXY_SEGMENTS / GALAXY_BATCH_SIZE
            );
        }

        let mut expected_stars = MeshBuffers::default();
        append_star_mesh_range(
            &mut expected_stars,
            &stars,
            0,
            stars.len(),
            latitude_deg,
            sidereal_time_deg,
            julian_day,
        );
        let mut expected_galaxy = MeshBuffers::default();
        append_galaxy_mesh_range(
            &mut expected_galaxy,
            0,
            GALAXY_SEGMENTS,
            latitude_deg,
            sidereal_time_deg,
        );

        assert_eq!(build.stars.positions, expected_stars.positions);
        assert_eq!(build.stars.normals, expected_stars.normals);
        assert_eq!(build.stars.colors, expected_stars.colors);
        assert_eq!(build.stars.indices, expected_stars.indices);
        assert_eq!(build.galaxy.positions, expected_galaxy.positions);
        assert_eq!(build.galaxy.normals, expected_galaxy.normals);
        assert_eq!(build.galaxy.colors, expected_galaxy.colors);
        assert_eq!(build.galaxy.indices, expected_galaxy.indices);
    }
}
