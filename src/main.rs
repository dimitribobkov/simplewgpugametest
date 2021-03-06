//#![windows_subsystem = "windows"] // Disable console


extern crate clap;
extern crate num;
use clap::{Arg, App};


use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};


use log4rs::{
    append::{
        file::FileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use wrapped2d::b2;
use wrapped2d::user_data::NoUserData;


mod renderer;
mod component;
mod transform;
mod input_manager;
mod entity;
mod system;
mod physics;
mod audio;
mod scene;

use renderer::renderer::Renderer;
use renderer::vertex::Vertex;
use renderer::texture::{Texture, DepthTexture, TextureMode};
use renderer::material::{Material, MaterialUniform};
use renderer::postprocessing::{PostProcessing, BloomUniform};
use input_manager::input_manager::InputManager;
use entity::rendermesh::RenderMesh;
use entity::entity::Entity;
use entity::entitymanager::EntityManager;
use renderer::camera::camera::{Camera, CameraUniform};
use renderer::camera::cameracontroller::CameraController;
use renderer::uniforms::{UniformUtils, UniformBuffer};
use renderer::uniforms::base_uniforms::BaseUniforms;
use component::ComponentBase;
use transform::{Translation, Rotation, NonUniformScale, Transform, TransformUniform};
use system::SystemBase;
use system::movement_system::MovementSystem;
use system::player_movement_system::PlayerMovementSystem;
use system::systemmanager::SystemManager;
use system::physics_system::PhysicsSystem;
use scene::SceneLoader;
use component::movement_component::MovementComponent;
use component::player_movement_component::PlayerMovementComponent;
use physics::physicscomponent::PhysicsComponent;
use physics::{Physics, PhysicsFilter, LayerType};
use audio::{Audio};
type World = b2::World<PhysicsFilter>;


use std::rc::Rc;
use std::cell::RefCell;

use futures::executor::block_on;


const TITLE: &str = "WGPU APP";

pub fn main() {
    let mut time = std::time::SystemTime::now();


    let matches = App::new(TITLE)
                          .version("1.0")
                          .author("Dimitri Bobkov <bobkov.dimitri@gmail.com>")
                          .about("Simple game renderer")
                          .arg(Arg::with_name("backend")
                               .short("b")
                               .long("backend")
                               .help("Sets a custom backend")
                               .takes_value(true))
                            .arg(Arg::with_name("vsync")
                               .short("v")
                               .long("vsync")
                               .help("Sets vysnc mode [on, off]")
                               .takes_value(true))
                          .arg(Arg::with_name("debug")
                                        .short("d")
                                        .long("debug")
                                        .help("Enable debug logging [full, info, warn, err, off]")
                                        .takes_value(true)
                                        .value_name("DEBUG_LEVEL"))
                          .arg(Arg::with_name("screenmode")
                                        .short("sm")
                                        .long("screenmode")
                                        .help("Set the screen mode: [full, borderless, windowed]")
                                        .takes_value(true)
                                        .value_name("SCREENMODE"))
                          .get_matches();

    let backend = matches.value_of("backend").unwrap_or("primary");
    backend.to_lowercase();
    
    let mut level = log::LevelFilter::Info;

    if matches.is_present("debug") {
        level = match matches.value_of("debug").unwrap_or("info"){
            "full" => log::LevelFilter::Trace,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "err" => log::LevelFilter::Error,
            "off" => {log::LevelFilter::Off},
            _ => log::LevelFilter::Info,
        };
    }
    let file_path = "./logs/output.log";
    let file_path_copy = "./logs/output_old.log";

    match std::fs::copy(file_path, file_path_copy){
        Ok(_) => {},
        Err(_) => {},
    };

    match std::fs::remove_file(file_path){
        Ok(_) => {},
        Err(_) => {},
    };

    // Logging to log file.
    let logfile = FileAppender::builder()
        // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
        .encoder(Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)}| {t}: {l} - {m}\n")))
        .build(file_path)
        .unwrap();

    // Log Trace level output to file where trace is the default level
    // and the programmatically specified level to stderr.
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(level),
        )
        .unwrap();

    // Use this to change log levels at runtime.
    // This means you can change the default log level to trace
    // if you are trying to debug an issue and need more logs on then turn it off
    // once you are done.
    let _handle = log4rs::init_config(config).unwrap();

    log::info!("Hello, world - Logger initalized!");


    log::info!("Logging Level: {:?}", level);

    let mut vsync_enabled = "true";

    vsync_enabled = match matches.value_of("vsync").unwrap_or("true"){
        "on" | "true" => "true",
        "off" | "false" => "false",
        "mail" | "mailbox" => "mailbox",
        _ => "true",
    };

    let mut screenmode = "full";

    screenmode = match matches.value_of("screenmode").unwrap_or("full"){
        "full" | "fullscreen" => "full",
        "border" | "borderless" | "fullscreenwindow" => "borderless",
        "win" | "windowed" | "window" => "windowed",
        _ => "full"
    };

    log::info!("Screen mode: {:?}", screenmode);

    let vsync_mode = match vsync_enabled{
        "true" => wgpu::PresentMode::Fifo,
        "false" => wgpu::PresentMode::Immediate,
        "mailbox" => wgpu::PresentMode::Mailbox,
        _ => wgpu::PresentMode::Mailbox
    };

    log::info!("Vsync Mode: {:?}", vsync_mode);
    

    if cfg!(debug_assertions) {
        println!("RUNNING: Debug");
        log::info!("App is running in debug mode");
    }else{
        println!("RUNNING: Release");
        log::info!("App is running in release mode");
    }


    // Actual program starts here

    /* Controls Audio */
    let mut audio = Audio::new();

    audio.play("./data/audio/nggyu.mp3", 0.5);
    log::info!("Audio controller created");
    

    let event_loop = EventLoop::new();
    
    let mut x = 0;
    let mut monitor: Vec<winit::monitor::MonitorHandle> = event_loop.available_monitors().filter(|_| if x == 0 { x += 1; true }else{ false }).collect();
    let monitor = monitor.swap_remove(0);
    
    let mut x = 0;
    let mut video_modes: Vec<winit::monitor::VideoMode> = monitor.video_modes().filter(|_| if x == 0 { x += 1; true }else{ false }).collect();
    let video_modes = video_modes.swap_remove(0);

    let window: winit::window::Window = match screenmode{
        "full" => {
            WindowBuilder::new()
            .with_fullscreen(Some(winit::window::Fullscreen::Exclusive(video_modes)))
            .with_title(TITLE)        
            .with_transparent(false)
            .build(&event_loop)
            .unwrap()
        },
        "borderless" => {
            WindowBuilder::new()
            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(monitor))))
            .with_title(TITLE)        
            .with_transparent(false)
            .build(&event_loop)
            .unwrap()
        },
        "windowed" => {
            WindowBuilder::new()
            .with_inner_size(winit::dpi::Size::from(winit::dpi::LogicalSize{ width: 1280, height: 720}))
            .with_title(TITLE)        
            .with_transparent(false)
            .build(&event_loop)
            .unwrap()
        },
        _ => {panic!()}
    };

    log::info!("Window created");


    let mut renderer;
    #[cfg(not(target_arch = "wasm32"))]
    {
        renderer = block_on(Renderer::new(&window, &backend, vsync_mode));
    }
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("could not initialize logger");
        use winit::platform::web::WindowExtWebSys;
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
        renderer = wasm_bindgen_futures::spawn_local(Renderer::new(&window, &backend));
    }
    let renderer = Rc::new(RefCell::new(renderer));
    log::info!("Renderer created");



    /* User Defined */
    let mut system_manager = SystemManager::new();
    let mut entity_manager = EntityManager::new();
    let mut input_manager = InputManager::new();
    let mut physics_manager = Physics::new();

    let movement_system = MovementSystem::new();
    let player_movement_system = PlayerMovementSystem::new();
    let physics_system = PhysicsSystem::new();

    log::info!("Systems successfully initialized");


    system_manager.add_system(Box::new(movement_system));
    system_manager.add_system(Box::new(player_movement_system));
    system_manager.add_system(Box::new(physics_system));


    // Since we share the renderer around, borrow it mutably
    let mut temp_renderer = renderer.borrow_mut();

    // Camera
    let mut camera_controller = CameraController::new(2.0);

    let sc_desc = &temp_renderer.sc_desc;

    let mut camera = Camera::new(
        &temp_renderer,
        // position the camera one unit up and 2 units back
        // +z is out of the screen
        (0.0, 0.0, 10.0).into(),
        // have it look at the origin
        (0.0, 0.0, 0.0).into(),
        // which way is "up"
        cgmath::Vector3::unit_y(),
        sc_desc.width as f32 / sc_desc.height as f32,
        45.0,
        0.1,
        25.0,
    );

    let (camera_bind_group, camera_layout, mut cam_uniform) = camera.create_uniforms(&temp_renderer);
    let camera_bind_group = Rc::new(camera_bind_group);


    // load textures (Define the texture layout)
    let texture_layout = Texture::generate_texture_layout(&temp_renderer);

    let white_texture = Rc::new(Texture::load_texture(&temp_renderer, "./data/textures/white.png", TextureMode::RGB).unwrap());
    let player_tex = Rc::new(Texture::load_texture(&temp_renderer, "./data/textures/player.png", TextureMode::RGBA).unwrap());
    let derp_texture = Rc::new(Texture::load_texture(&temp_renderer, "./data/textures/derp.png", TextureMode::RGBA).unwrap());
    let pepe_texture = Rc::new(Texture::load_texture(&temp_renderer, "./data/textures/pepe.png", TextureMode::RGBA).unwrap());
    
    // Create transform layout
    let transform_layout = UniformUtils::create_bind_group_layout(&temp_renderer, 0, wgpu::ShaderStage::VERTEX, Some("Transform"));


    // define the array with layouts we want to use in our pipeline
    let mut layouts = vec!(&texture_layout, &camera_layout);
    // create material
    let material_layout = Material::create_uniform_layout(&temp_renderer);
    layouts.push(&material_layout);
    layouts.push(&transform_layout);




    /*let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material - lower depth is sorted higher
    let material = Material::new(&temp_renderer, Rc::clone(&player_tex), cgmath::Vector3::<f32> { x: 0.0, y: 500.0, z: 500.0 }, 1.0, 0.0, 0, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = material_group;

    let translation = Translation::new(cgmath::Vector3::<f32> { x: 0.0, y: 0.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0});

    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);
    let pmc = PlayerMovementComponent::new(15.0);
    let phs_comp = PhysicsComponent::new_box(&mut physics_manager, transform.position, (0.2, 1.0), 1.0, b2::BodyType::Dynamic, 1, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::new(material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(pmc));
    components.push(Box::new(phs_comp));


    {
        entity_manager.create_entity(components, uniforms);
    }*/





    let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material
    let material = Material::new(&temp_renderer, Rc::clone(&derp_texture), cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0 }, 1.0, 0.0, -1, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = Rc::new(material_group);

    let translation = Translation::new(cgmath::Vector3::<f32> { x: 3.0, y: 2.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0});


    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);

    let phs_comp = PhysicsComponent::new_circle(&mut physics_manager, transform.position, 1.0, 5.0, b2::BodyType::Dynamic, 0, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::clone(&material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(phs_comp));


    {
        entity_manager.create_entity(components, uniforms);
    }





    let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material
    let material = Material::new(&temp_renderer, Rc::clone(&pepe_texture), cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0 }, 1.0, 0.0, -1, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = Rc::new(material_group);

    let translation = Translation::new(cgmath::Vector3::<f32> { x: -2.0, y: 1.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0});


    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);

    let phs_comp = PhysicsComponent::new_box(&mut physics_manager, transform.position, (1.0, 1.0), 2.0, b2::BodyType::Dynamic, 0, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::clone(&material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(phs_comp));


    {

        entity_manager.create_entity(components, uniforms);
        let new_entity = entity_manager.find_entity(0).unwrap();

        entity_manager.add_component_data::<MovementComponent>(&new_entity, Box::new(MovementComponent::new(-75.0))).unwrap();
        let component = entity_manager.get_component_data::<Transform>(new_entity, Transform::get_component_id()).unwrap();
        component.position = cgmath::Vector3::<f32> { x: 2.5, y: -1.0, z: 0.0 };
        component.update_uniform_buffers(&temp_renderer);

        //drop(component);

        let new_entity = entity_manager.find_entity(1).unwrap();
        entity_manager.add_component_data::<MovementComponent>(&new_entity, Box::new(MovementComponent::new(-75.0))).unwrap();
    }






    let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material
    let material = Material::new(&temp_renderer, Rc::clone(&white_texture), cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0 }, 1.0, 0.0, -1, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = Rc::new(material_group);

    let translation = Translation::new(cgmath::Vector3::<f32> { x: 0.0, y: -5.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 20.0, y: 1.0, z: 1.0});


    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);

    let phs_comp = PhysicsComponent::new_box(&mut physics_manager, transform.position, (20.0, 1.0), 0.0, b2::BodyType::Static, 2, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::clone(&material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(phs_comp));


    {
        entity_manager.create_entity(components, uniforms);
    }




    let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material
    let material = Material::new(&temp_renderer, Rc::clone(&white_texture), cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0 }, 1.0, 0.0, -1, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = Rc::new(material_group);

    let translation = Translation::new(cgmath::Vector3::<f32> { x: 45.0, y: -3.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 20.0, y: 1.0, z: 1.0});


    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);

    let phs_comp = PhysicsComponent::new_box(&mut physics_manager, transform.position, (20.0, 1.0), 0.0, b2::BodyType::Static, 2, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::clone(&material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(phs_comp));


    {
        entity_manager.create_entity(components, uniforms);
    }


    let mut uniforms = Vec::<Rc<wgpu::BindGroup>>::new();
    let mut components = Vec::<Box<dyn ComponentBase>>::new();

    // create material
    let material = Material::new(&temp_renderer, Rc::clone(&white_texture), cgmath::Vector3::<f32> { x: 1.0, y: 1.0, z: 1.0 }, 1.0, 0.0, -1, "main".to_string());

    // create new mesh (TODO - mesh loading) and assign material
    let mut mesh = RenderMesh::new(&temp_renderer, material);
    let (material_group, _, _) = mesh.generate_material_uniforms(&temp_renderer);
    let material_group = Rc::new(material_group);

    let translation = Translation::new(cgmath::Vector3::<f32> { x: -45.0, y: -2.0, z: 0.0});

    let rotation = Rotation::new(cgmath::Quaternion::from(cgmath::Euler {
        x: cgmath::Deg(0.0),
        y: cgmath::Deg(0.0),
        z: cgmath::Deg(0.0),
    }));

    let scale = NonUniformScale::new(cgmath::Vector3::<f32> { x: 20.0, y: 1.0, z: 1.0});


    let mut transform = Transform::new(&temp_renderer, translation.value, rotation.value, scale.value);
    let (transform_group, _, _) = transform.create_uniforms(&temp_renderer);
    let transform_group = Rc::new(transform_group);

    let phs_comp = PhysicsComponent::new_box(&mut physics_manager, transform.position, (20.0, 1.0), 0.0, b2::BodyType::Static, 2, false);


    uniforms.push(Rc::clone(&camera_bind_group));
    uniforms.push(Rc::clone(&material_group));
    uniforms.push(Rc::clone(&transform_group));

    components.push(Box::new(mesh));
    components.push(Box::new(transform));
    components.push(Box::new(phs_comp));


    {
        entity_manager.create_entity(components, uniforms);
    }


    println!("Entity Count: {:?}", entity_manager.entities.len());

    
    SceneLoader::load("./data/scene/scene.dbscene", &mut entity_manager, &mut physics_manager, &temp_renderer, Rc::clone(&camera_bind_group));

    println!("Entity Count: {:?}", entity_manager.entities.len());

    log::info!("Entities built");

    // Create all our render pipelines. Define the color states (This is like instructions to  the GPU on how to blend the color, alpha and color format etc)
    // Important for render passes. One color state per output.
    let mut color_states = Vec::<wgpu::ColorStateDescriptor>::new();


    // Define the color states for the main pass render pipeline. We need one per color attachment
    color_states.push(wgpu::ColorStateDescriptor {
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        color_blend: wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add
        },
        alpha_blend: wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add
        },
        //color_blend: wgpu::BlendDescriptor::REPLACE,
        //alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL
    });

    color_states.push(wgpu::ColorStateDescriptor {
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        color_blend: wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add
        },
        alpha_blend: wgpu::BlendDescriptor {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add
        },
        //color_blend: wgpu::BlendDescriptor::REPLACE,
        //alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL
    });

    // recreate pipeline with layouts (needs mut)
    temp_renderer.create_pipeline("main".to_string(), &layouts, wgpu::include_spirv!("./shaders/shader.vert.spv"), wgpu::include_spirv!("./shaders/shader.frag.spv"), &color_states, &[Vertex::desc()], 1, true);
    layouts.clear();

    /* Screen based rendering now */
    let main_tex_layout = &Texture::generate_texture_layout_from_device(&temp_renderer.device);
    let hdr_tex_layout = &Texture::generate_texture_layout_from_device(&temp_renderer.device);
    let shadow_tex_layout = &Texture::generate_texture_layout_from_device(&temp_renderer.device);
    layouts.push(main_tex_layout);
    layouts.push(hdr_tex_layout);

    layouts.remove(1);

    color_states.clear();

    color_states.push(wgpu::ColorStateDescriptor {
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
    });



    let bloom_u_layout = &BloomUniform::create_uniform_layout(&temp_renderer);
    layouts.push(bloom_u_layout);
    
    temp_renderer.create_pipeline("bloom".to_string(), &layouts, wgpu::include_spirv!("./shaders/framebuffer.vert.spv"), wgpu::include_spirv!("./shaders/bloom.frag.spv"), &color_states, &[], 1, false);

    layouts.remove(1);

    let fb_u_layout = &BaseUniforms::create_uniform_layout(&temp_renderer);

    layouts.push(hdr_tex_layout);
    layouts.push(fb_u_layout);


    temp_renderer.create_pipeline("fxaa".to_string(), &layouts, wgpu::include_spirv!("./shaders/framebuffer.vert.spv"), wgpu::include_spirv!("./shaders/fxaa.frag.spv"), &color_states, &[], 1, false);





    color_states.clear();

    // Define the color states for the framebuffer render pipeline. We need one per color attachment
    color_states.push(wgpu::ColorStateDescriptor {
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        color_blend: wgpu::BlendDescriptor::REPLACE,
        alpha_blend: wgpu::BlendDescriptor::REPLACE,
        write_mask: wgpu::ColorWrite::ALL,
    });


    let sample_count = temp_renderer.sample_count;
    temp_renderer.create_pipeline("framebuffer".to_string(), &layouts, wgpu::include_spirv!("./shaders/framebuffer.vert.spv"), wgpu::include_spirv!("./shaders/framebuffer.frag.spv"), &color_states, &[], sample_count, false);
   
    layouts.clear();
    color_states.clear();


    log::info!("Render Pipelines built");
    
    // drop the borrowed mut reference (to stay safe)
    drop(temp_renderer);

    /* Define some runtime variables */
    let mut framerate: f32 = 0.0;

    /* Game Loop Defined */

    log::info!("Starting main loop");
    event_loop.run(move |event, _, control_flow|  
        match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() =>  {
            //camera_controller.process_events(event);
            input_manager.update(event);
            let mut renderer = renderer.borrow_mut();
            match event{
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput {
                input,
                ..
            } => {
                match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => {    log::info!("User Quit Application"); *control_flow = ControlFlow::Exit},
                    _ => {}
                }
            },
            WindowEvent::Resized(physical_size) => {
                renderer.resize(*physical_size);
                let sc_desc = &renderer.sc_desc;
                camera.aspect = sc_desc.width as f32 / sc_desc.height as f32;
                log::info!("User resized screen");
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                // new_inner_size is &&mut so we have to dereference it twice
                renderer.resize(**new_inner_size);
                let sc_desc = &renderer.sc_desc;
                camera.aspect = sc_desc.width as f32 / sc_desc.height as f32;
                log::info!("User resized screen");
            },
            
            _ => {}
        }
        },
        Event::RedrawRequested(_) => {

            let start = std::time::SystemTime::now();
            let window_size = renderer.borrow().get_window_size();
            let mut renderer = renderer.borrow_mut();
 

            //camera_controller.update_camera(&mut camera);
            cam_uniform.update_view_proj(&mut camera, &renderer.sc_desc);
            renderer.write_buffer(camera.get_buffer_reference(), 0, &[cam_uniform]);         
        
            system_manager.update_systems(&renderer, &mut entity_manager,  &input_manager, &mut physics_manager, &mut camera);
            renderer.update();
            


            match renderer.render(&mut camera, &entity_manager, &time, framerate) {
                Ok(_) => {}
                // Recreate the swap_chain if lost
                Err(wgpu::SwapChainError::Lost) => renderer.resize(window_size),
                // The system is out of memory, we should probably quit
                Err(wgpu::SwapChainError::OutOfMemory) => {*control_flow = ControlFlow::Exit; log::error!("Device out of memory!")},
                // All other errors (Outdated, Timeout) should be resolved by the next frame
                Err(e) => {eprintln!("{:?}", e); log::error!("{:?}", e)},
            }

            let delta_time = start.elapsed().unwrap().as_secs_f32();
            system_manager.delta_time = delta_time;
            camera_controller.delta_time = delta_time;
            if ((1.0 / delta_time) - framerate).abs() > 2.0{
                framerate = 1.0 / delta_time;
            }
        }
        Event::MainEventsCleared => {
            // RedrawRequested will only trigger once, unless we manually
            // request it.
            window.request_redraw();
        }
        _ => {}
    }
    
);
}


