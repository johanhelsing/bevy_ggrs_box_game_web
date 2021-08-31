use bevy::{core::FixedTimestep, prelude::*};
use std::net::SocketAddr;

use bevy_ggrs::{GGRSApp, GGRSPlugin};
use ggrs::PlayerType;

mod args;
mod box_game;

use args::*;
use box_game::*;

const INPUT_SIZE: usize = std::mem::size_of::<u8>();
const FPS: u32 = 60;

fn main() {
    // When building for WASM, print panics to the browser console
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    real_main().expect("FAILED");
}

fn real_main() -> Result<(), Box<dyn std::error::Error>> {
    // read cmd line arguments
    let opt = Args::get();

    let mut local_handle = 0;
    let num_players = opt.players.len();
    assert!(num_players > 0);

    // create a GGRS P2P session
    let mut p2p_sess = ggrs::start_p2p_session(num_players as u32, INPUT_SIZE, opt.local_port)?;

    // turn on sparse saving
    p2p_sess.set_sparse_saving(true)?;

    // add players
    for (i, player_addr) in opt.players.iter().enumerate() {
        // local player
        if player_addr == "localhost" {
            p2p_sess.add_player(PlayerType::Local, i)?;
            local_handle = i;
        } else {
            // remote players
            let remote_addr: SocketAddr = player_addr.parse()?;
            p2p_sess.add_player(PlayerType::Remote(remote_addr), i)?;
        }
    }
    // optionally, add spectators
    for (i, spec_addr) in opt.spectators.iter().enumerate() {
        p2p_sess.add_player(PlayerType::Spectator(*spec_addr), num_players + i)?;
    }

    // set input delay for the local player
    p2p_sess.set_frame_delay(2, local_handle)?;

    // set default expected update frequency (affects synchronization timings between players)
    p2p_sess.set_fps(FPS)?;

    // start the GGRS session
    p2p_sess.start_session()?;

    let mut app = App::new();

    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);

    app.add_startup_system(setup_system.system());

    app.add_plugin(GGRSPlugin);

    // add your GGRS session
    app.with_p2p_session(p2p_sess);
    // define frequency of game logic update
    app.with_rollback_run_criteria(FixedTimestep::steps_per_second(FPS as f64))
        // define system that represents your inputs as a byte vector, so GGRS can send the inputs around
        .with_input_system(input.system())
        // register components that will be loaded/saved
        .register_rollback_type::<Transform>()
        .register_rollback_type::<Velocity>()
        // you can also register resources
        .insert_resource(FrameCount { frame: 0 })
        .register_rollback_type::<FrameCount>()
        // these systems will be executed as part of the advance frame update
        .add_rollback_system(move_cube_system)
        .add_rollback_system(increase_frame_system);

    app.run();

    Ok(())
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_translation(Vec3::new(0.0, 0.5, 0.0)),
        ..Default::default()
    });
    // light
    // commands.spawn_bundle(LightBundle {
    //     transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
    //     ..Default::default()
    // });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(Vec3::new(-2.0, 2.5, 5.0))
            .looking_at(Vec3::default(), Vec3::Y),
        ..Default::default()
    });
}
