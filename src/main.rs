mod gpu;

use gpu::{GpuInfo, GpuKind};
use gtk::gdk;
use gtk::prelude::*;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::rc::Rc;

const AD_BLOCK_RULES: &str = "MAP quge5.com ~NOTFOUND, MAP *.quge5.com ~NOTFOUND, MAP alwingulla.com ~NOTFOUND, MAP *.alwingulla.com ~NOTFOUND";

const CSS: &str = r#"
window { background: #080b10; color: #f4f7f8; }
.custom-titlebar { background: #0b0f14; border-bottom: 1px solid rgba(255,255,255,0.08); min-height: 48px; padding: 0 10px 0 16px; }
.title-mark { background: #71e6a6; color: #07140d; border-radius: 8px; font-weight: 1000; min-width: 28px; min-height: 28px; }
.window-control { background: transparent; border: 0; border-radius: 8px; min-width: 34px; min-height: 32px; padding: 0; }
.window-control:hover { background: rgba(255,255,255,0.10); }
.close-control:hover { background: #d94f5c; color: white; }
.app-title { font-weight: 800; font-size: 15px; }
.shell { padding: 34px; }
.hero { background-image: linear-gradient(135deg, #172128, #102e25); border: 1px solid rgba(116,232,166,0.24); border-radius: 24px; padding: 30px; }
.eyebrow { color: #75e7a7; font-size: 11px; font-weight: 800; letter-spacing: 2px; }
.hero-title { font-size: 31px; font-weight: 900; }
.hero-copy { color: #b8c3c8; font-size: 14px; }
.v-badge { background: #75e7a7; color: #07130c; border-radius: 18px; font-size: 32px; font-weight: 1000; min-width: 72px; min-height: 72px; }
.section-title { font-size: 13px; font-weight: 800; color: #d8dfe2; }
.feature-card { background: #11161d; border: 1px solid rgba(255,255,255,0.08); border-radius: 17px; padding: 17px; }
.feature-icon { color: #75e7a7; }
.feature-title { font-weight: 800; font-size: 14px; }
.feature-copy { color: #919da3; font-size: 12px; }
.ready-copy { color: #71e6a6; font-size: 12px; font-weight: 700; }
.warning-copy { color: #f0bf68; font-size: 12px; font-weight: 700; }
.settings-card { background: #0e1319; border: 1px solid rgba(255,255,255,0.07); border-radius: 18px; padding: 8px 18px; }
.setting-row { padding: 11px 2px; }
.setting-title { font-weight: 700; }
.setting-copy { color: #89959b; font-size: 12px; }
switch:checked { background: #58d993; }
dropdown, entry { background: #171e26; border: 1px solid rgba(255,255,255,0.10); border-radius: 10px; min-height: 36px; }
.play-button { background: #71e6a6; color: #07140d; border-radius: 15px; min-height: 52px; font-size: 16px; font-weight: 900; box-shadow: 0 8px 24px rgba(83,217,145,0.18); }
.play-button:hover { background: #83edb3; }
.status-copy { color: #829096; font-size: 12px; }
.error-copy { color: #ff8f8f; font-size: 12px; }
"#;

#[derive(Clone, Debug)]
struct Options {
    ad_block: bool,
    browser: Option<PathBuf>,
    direct_launch: bool,
    dry_run: bool,
    fullscreen: bool,
    graphics_check: bool,
    integrated_gpu: bool,
    performance: PerformanceMode,
    selected_gpu: Option<String>,
    url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PerformanceMode {
    Balanced,
    Performance,
    MemorySaver,
}

impl PerformanceMode {
    fn from_index(index: u32) -> Self {
        match index {
            1 => Self::Performance,
            2 => Self::MemorySaver,
            _ => Self::Balanced,
        }
    }

    fn renderer_limit(self) -> u8 {
        match self {
            Self::Balanced => 6,
            Self::Performance => 8,
            Self::MemorySaver => 4,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fiveplayonlinux: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = parse_args()?;
    if options.graphics_check {
        print_graphics_check()?;
        return Ok(());
    }
    let browser = match &options.browser {
        Some(path) => resolve_executable(path)
            .ok_or_else(|| format!("browser executable not found: {}", path.display()))?,
        None => find_browser().ok_or_else(|| {
            "no supported Chromium browser found (tried Microsoft Edge, Chromium, and Brave)"
                .to_owned()
        })?,
    };
    let profile = profile_dir()?;
    fs::create_dir_all(&profile)
        .map_err(|error| format!("cannot create {}: {error}", profile.display()))?;

    if options.dry_run {
        print_command(&game_command(&browser, &profile, &options)?);
        return Ok(());
    }
    if options.direct_launch {
        return exec_game(game_command(&browser, &profile, &options)?, &browser);
    }
    run_interface(browser, profile, options);
    Ok(())
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        ad_block: true,
        browser: None,
        direct_launch: false,
        dry_run: false,
        fullscreen: load_saved_value("fullscreen").as_deref() == Some("true"),
        graphics_check: false,
        integrated_gpu: false,
        performance: match load_saved_value("performance").as_deref() {
            Some("performance") => PerformanceMode::Performance,
            Some("memory-saver") => PerformanceMode::MemorySaver,
            _ => PerformanceMode::Balanced,
        },
        selected_gpu: load_saved_value("graphics-device"),
        url: load_saved_url().unwrap_or_default(),
    };
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--browser") => {
                options.browser = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--browser requires a path".to_owned())?,
                ));
            }
            Some("--url") => {
                options.url = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--url requires a valid UTF-8 URL".to_owned())?;
            }
            Some("--dry-run") => options.dry_run = true,
            Some("--fullscreen") => options.fullscreen = true,
            Some("--graphics-check") => options.graphics_check = true,
            Some("--integrated") => options.integrated_gpu = true,
            Some("--performance") => options.performance = PerformanceMode::Performance,
            Some("--memory-saver") => options.performance = PerformanceMode::MemorySaver,
            Some("--no-adblock") => options.ad_block = false,
            Some("--play") => options.direct_launch = true,
            Some("-h" | "--help") => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    if !options.url.is_empty() {
        validate_url(&options.url)?;
    }
    Ok(options)
}

fn print_help() {
    println!(
        "fiveplayonlinux {version}\n\
         Native Play V2 Fiveplay client for Linux.\n\n\
         Usage: fiveplayonlinux [OPTIONS]\n\n\
         Options:\n\
           --play            Skip the interface and launch immediately\n\
           --fullscreen      Start the game in fullscreen mode\n\
           --graphics-check  Print the Vulkan compatibility report\n\
           --performance     Prefer maximum foreground performance\n\
           --memory-saver    Limit browser processes to reduce memory\n\
           --no-adblock      Allow the site's advertising hosts\n\
           --browser PATH    Use a specific Chromium-family executable\n\
           --url URL         Use a secure Play V2 launch address\n\
           --integrated      Prefer the integrated GPU\n\
           --dry-run         Print the resolved launch command\n\
           -h, --help        Show this help\n",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn print_graphics_check() -> Result<(), String> {
    let devices = gpu::probe()?;
    let compatible = gpu::compatible_devices(&devices);
    if devices.is_empty() {
        return Err("No Vulkan graphics devices were detected.".to_owned());
    }
    for device in &devices {
        println!(
            "{} | {} | {} | {}",
            device.name,
            device.kind.label(),
            device.memory_label(),
            device.readiness()
        );
    }
    match compatible.first() {
        Some(device) => println!("Recommended: {}", device.name),
        None => return Err("No compatible Vulkan graphics device was found.".to_owned()),
    }
    Ok(())
}

fn game_command(browser: &Path, profile: &Path, options: &Options) -> Result<Command, String> {
    let devices = gpu::probe()?;
    let selected = gpu::select(
        &devices,
        options.selected_gpu.as_deref(),
        options.integrated_gpu,
    )
    .ok_or_else(|| "No compatible Vulkan graphics device was found.".to_owned())?;
    game_command_for_gpu(browser, profile, options, &selected)
}

fn game_command_for_gpu(
    browser: &Path,
    profile: &Path,
    options: &Options,
    selected_gpu: &GpuInfo,
) -> Result<Command, String> {
    validate_url(&options.url)?;
    let mut command = Command::new(browser);
    add_browser_args(&mut command, profile, options);
    configure_gpu(&mut command, selected_gpu);
    Ok(command)
}

fn add_browser_args(command: &mut Command, profile: &Path, options: &Options) {
    command
        .arg(format!("--app={}", options.url))
        .arg(format!("--user-data-dir={}", profile.display()))
        .arg("--class=FiveplayOnLinux")
        .arg("--name=FiveplayOnLinux")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg("--disable-background-networking")
        .arg("--disable-background-mode")
        .arg("--disable-component-update")
        .arg("--disable-extensions")
        .arg("--disable-sync")
        .arg("--enable-gpu-rasterization")
        .arg("--enable-unsafe-webgpu")
        .arg("--enable-zero-copy")
        .arg("--ignore-gpu-blocklist")
        .arg(format!(
            "--renderer-process-limit={}",
            options.performance.renderer_limit()
        ))
        .arg("--ozone-platform-hint=auto");
    if options.performance == PerformanceMode::Performance {
        command
            .arg("--disable-background-timer-throttling")
            .arg("--disable-renderer-backgrounding")
            .arg("--disable-backgrounding-occluded-windows");
    }
    if options.fullscreen {
        command.arg("--start-fullscreen");
    }
    if options.ad_block {
        command.arg(format!("--host-resolver-rules={AD_BLOCK_RULES}"));
    }
}

fn configure_gpu(command: &mut Command, gpu: &GpuInfo) {
    if gpu.kind.is_discrete() {
        command.env("DRI_PRIME", "1");
        if gpu.vendor_id == 0x10de {
            command
                .env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
                .env("__NV_PRIME_RENDER_OFFLOAD", "1")
                .env("__VK_LAYER_NV_optimus", "NVIDIA_only")
                .env("VK_LOADER_DRIVERS_SELECT", "*nvidia*");
        } else {
            command.env(
                "MESA_VK_DEVICE_SELECT",
                format!("{:04x}:{:04x}!", gpu.vendor_id, gpu.device_id),
            );
        }
    } else if matches!(gpu.kind, GpuKind::Integrated) {
        command.env("DRI_PRIME", "0");
    }
}

fn exec_game(mut command: Command, browser: &Path) -> Result<(), String> {
    let error = command.exec();
    Err(format!("failed to start {}: {error}", browser.display()))
}

fn run_interface(browser: PathBuf, profile: PathBuf, options: Options) {
    let application = gtk::Application::builder()
        .application_id("io.github.Knownassa.FIveplayonlinux")
        .build();
    application.connect_startup(|_| install_css());
    application.connect_activate(move |application| {
        build_window(
            application,
            browser.clone(),
            profile.clone(),
            options.clone(),
        );
    });
    application.run();
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(
    application: &gtk::Application,
    browser: PathBuf,
    profile: PathBuf,
    options: Options,
) {
    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("Play V2 Fiveplay")
        .default_width(880)
        .default_height(760)
        .resizable(true)
        .build();
    window.set_titlebar(Some(&custom_titlebar(&window)));

    let (compatible_gpus, graphics_message) = match gpu::probe() {
        Ok(devices) => {
            let compatible = gpu::compatible_devices(&devices);
            let message = if let Some(recommended) = compatible.first() {
                format!(
                    "{} selected · {}",
                    recommended.name,
                    recommended.readiness()
                )
            } else {
                "No compatible Vulkan graphics device was found.".to_owned()
            };
            (compatible, message)
        }
        Err(error) => (Vec::new(), error),
    };
    let compatible_gpus = Rc::new(compatible_gpus);

    let shell = gtk::Box::new(gtk::Orientation::Vertical, 22);
    shell.add_css_class("shell");
    let hero = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    hero.add_css_class("hero");
    hero.set_hexpand(true);
    let hero_text = gtk::Box::new(gtk::Orientation::Vertical, 7);
    hero_text.set_hexpand(true);
    hero_text.set_valign(gtk::Align::Center);
    let eyebrow = label("FIVEPLAYONLINUX  /  PERFORMANCE CLIENT", "eyebrow");
    let title = label("Play V2. Clean and accelerated.", "hero-title");
    let copy = label(
        if compatible_gpus.is_empty() {
            "Graphics preflight needs attention before the game can start."
        } else {
            "A clean WebGPU session with automatic graphics-device validation."
        },
        "hero-copy",
    );
    hero_text.append(&eyebrow);
    hero_text.append(&title);
    hero_text.append(&copy);
    let badge = gtk::Label::new(Some("V"));
    badge.set_halign(gtk::Align::Center);
    badge.set_valign(gtk::Align::Center);
    badge.add_css_class("v-badge");
    hero.append(&hero_text);
    hero.append(&badge);
    shell.append(&hero);

    let feature_title = label("READY TO PLAY", "section-title");
    shell.append(&feature_title);
    let features = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    features.set_homogeneous(true);
    features.append(&feature_card(
        "◈",
        "Graphics preflight",
        if compatible_gpus.is_empty() {
            "No compatible device"
        } else {
            "Vulkan device verified"
        },
    ));
    features.append(&feature_card(
        "◇",
        "Clean session",
        "Ad hosts blocked early",
    ));
    features.append(&feature_card("⚡", "WebGPU ready", "Acceleration enforced"));
    shell.append(&features);

    let settings = gtk::Box::new(gtk::Orientation::Vertical, 0);
    settings.add_css_class("settings-card");
    let address_entry = gtk::Entry::builder()
        .text(&options.url)
        .placeholder_text("Paste your secure launch address")
        .hexpand(false)
        .width_chars(32)
        .build();
    let gpu_labels: Vec<String> = compatible_gpus.iter().map(GpuInfo::display_label).collect();
    let gpu_label_refs: Vec<&str> = gpu_labels.iter().map(String::as_str).collect();
    let gpu_dropdown = gtk::DropDown::from_strings(&gpu_label_refs);
    gpu_dropdown.set_valign(gtk::Align::Center);
    gpu_dropdown.set_sensitive(!compatible_gpus.is_empty());
    let requested_gpu = options.selected_gpu.as_deref();
    let selected_index = requested_gpu
        .and_then(|name| {
            compatible_gpus
                .iter()
                .position(|device| device.name == name)
        })
        .or_else(|| {
            options.integrated_gpu.then(|| {
                compatible_gpus
                    .iter()
                    .position(|device| matches!(device.kind, GpuKind::Integrated))
                    .unwrap_or(0)
            })
        })
        .unwrap_or(0);
    if !compatible_gpus.is_empty() {
        gpu_dropdown.set_selected(selected_index as u32);
    }
    let graphics_detail = compatible_gpus
        .get(selected_index)
        .map(|device| format!("{} · {}", device.name, device.readiness()))
        .unwrap_or_else(|| graphics_message.clone());
    let graphics_detail_label = label(&graphics_detail, "setting-copy");
    let graphics_detail_for_change = graphics_detail_label.clone();
    let compatible_for_change = compatible_gpus.clone();
    gpu_dropdown.connect_selected_notify(move |dropdown| {
        if let Some(device) = compatible_for_change.get(dropdown.selected() as usize) {
            graphics_detail_for_change.set_text(&format!(
                "{} · {}",
                device.name,
                device.readiness()
            ));
        }
    });
    let performance_dropdown =
        gtk::DropDown::from_strings(&["Balanced", "High performance", "Memory saver"]);
    performance_dropdown.set_selected(match options.performance {
        PerformanceMode::Balanced => 0,
        PerformanceMode::Performance => 1,
        PerformanceMode::MemorySaver => 2,
    });
    performance_dropdown.set_valign(gtk::Align::Center);
    let fullscreen_switch = gtk::Switch::builder()
        .active(options.fullscreen)
        .valign(gtk::Align::Center)
        .build();
    let block_switch = gtk::Switch::builder()
        .active(options.ad_block)
        .valign(gtk::Align::Center)
        .build();
    settings.append(&setting_row(
        "Launch address",
        "Stored only in your local configuration, never in the project source.",
        &address_entry,
    ));
    settings.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    settings.append(&setting_row_with_detail(
        "Graphics device",
        &graphics_detail_label,
        &gpu_dropdown,
    ));
    settings.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    settings.append(&setting_row(
        "Session profile",
        "Performance keeps the game responsive; Memory saver reduces processes.",
        &performance_dropdown,
    ));
    settings.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    settings.append(&setting_row(
        "Start fullscreen",
        "Launch directly into a distraction-free fullscreen session.",
        &fullscreen_switch,
    ));
    settings.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    settings.append(&setting_row(
        "Block advertising hosts",
        "Stops external advertising scripts before they connect.",
        &block_switch,
    ));
    shell.append(&settings);

    let status = label(
        "The launcher closes when the game starts, leaving no UI process in memory.",
        "status-copy",
    );
    let play = gtk::Button::with_label("Play V2 Fiveplay");
    play.add_css_class("play-button");
    play.set_hexpand(true);
    play.set_sensitive(!compatible_gpus.is_empty());
    let browser_name = browser
        .file_name()
        .unwrap_or_else(|| OsStr::new("Chromium"))
        .to_string_lossy();
    let engine = gtk::Label::new(Some(&format!(
        "Engine: {browser_name}  •  Isolated profile"
    )));
    engine.set_halign(gtk::Align::Center);
    engine.add_css_class("status-copy");

    let status_for_click = status.clone();
    let window_for_click = window.clone();
    let compatible_for_click = compatible_gpus.clone();
    play.connect_clicked(move |_| {
        let selected_index = gpu_dropdown.selected() as usize;
        let Some(selected_gpu) = compatible_for_click.get(selected_index) else {
            status_for_click.add_css_class("error-copy");
            status_for_click.set_text("Select a compatible graphics device first.");
            return;
        };
        let launch_options = Options {
            ad_block: block_switch.is_active(),
            fullscreen: fullscreen_switch.is_active(),
            integrated_gpu: matches!(selected_gpu.kind, GpuKind::Integrated),
            performance: PerformanceMode::from_index(performance_dropdown.selected()),
            selected_gpu: Some(selected_gpu.name.clone()),
            url: address_entry.text().trim().to_owned(),
            ..options.clone()
        };
        if let Err(error) = validate_url(&launch_options.url) {
            status_for_click.add_css_class("error-copy");
            status_for_click.set_text(&error);
            return;
        }
        if let Err(error) = save_url(&launch_options.url) {
            status_for_click.add_css_class("error-copy");
            status_for_click.set_text(&error);
            return;
        }
        if let Err(error) = save_launch_preferences(&launch_options) {
            status_for_click.add_css_class("error-copy");
            status_for_click.set_text(&error);
            return;
        }
        status_for_click.set_text("Starting the WebGPU client…");
        status_for_click.remove_css_class("error-copy");
        window_for_click.set_sensitive(false);
        let mut command = match game_command(&browser, &profile, &launch_options) {
            Ok(command) => command,
            Err(error) => {
                window_for_click.set_sensitive(true);
                status_for_click.add_css_class("error-copy");
                status_for_click.set_text(&error);
                return;
            }
        };
        let error = command.exec();
        window_for_click.set_sensitive(true);
        status_for_click.add_css_class("error-copy");
        status_for_click.set_text(&format!("Could not start the game: {error}"));
    });
    shell.append(&play);
    shell.append(&status);
    shell.append(&engine);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&shell)
        .build();
    window.set_child(Some(&scroll));
    window.present();
}

fn custom_titlebar(window: &gtk::ApplicationWindow) -> gtk::WindowHandle {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("custom-titlebar");

    let mark = gtk::Label::new(Some("V"));
    mark.set_halign(gtk::Align::Center);
    mark.set_valign(gtk::Align::Center);
    mark.add_css_class("title-mark");
    let title = gtk::Label::new(Some("PLAY V2 FIVEPLAY"));
    title.set_hexpand(true);
    title.set_halign(gtk::Align::Start);
    title.add_css_class("app-title");

    let minimize = window_button("—", "Minimize");
    let maximize = window_button("□", "Maximize or restore");
    let close = window_button("×", "Close");
    close.add_css_class("close-control");

    let window_for_minimize = window.clone();
    minimize.connect_clicked(move |_| window_for_minimize.minimize());
    let window_for_maximize = window.clone();
    maximize.connect_clicked(move |_| {
        if window_for_maximize.is_maximized() {
            window_for_maximize.unmaximize();
        } else {
            window_for_maximize.maximize();
        }
    });
    let window_for_close = window.clone();
    close.connect_clicked(move |_| window_for_close.close());

    bar.append(&mark);
    bar.append(&title);
    bar.append(&minimize);
    bar.append(&maximize);
    bar.append(&close);
    gtk::WindowHandle::builder().child(&bar).build()
}

fn window_button(symbol: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::with_label(symbol);
    button.set_tooltip_text(Some(tooltip));
    button.set_focus_on_click(false);
    button.add_css_class("window-control");
    button
}

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    label.set_wrap(true);
    label.add_css_class(class);
    label
}

fn feature_card(symbol: &str, title: &str, copy: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 7);
    card.add_css_class("feature-card");
    let icon = gtk::Label::new(Some(symbol));
    icon.set_halign(gtk::Align::Start);
    icon.add_css_class("feature-icon");
    card.append(&icon);
    card.append(&label(title, "feature-title"));
    card.append(&label(copy, "feature-copy"));
    card
}

fn setting_row(title: &str, copy: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let detail = label(copy, "setting-copy");
    setting_row_with_detail(title, &detail, control)
}

fn setting_row_with_detail(
    title: &str,
    detail: &gtk::Label,
    control: &impl IsA<gtk::Widget>,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("setting-row");
    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    text.append(&label(title, "setting-title"));
    text.append(detail);
    row.append(&text);
    row.append(control);
    row
}

fn profile_dir() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(path).join("fiveplayonlinux").join("profile"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share/fiveplayonlinux/profile"))
        .ok_or_else(|| "neither XDG_DATA_HOME nor HOME is set".to_owned())
}

fn config_file(name: &str) -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("fiveplayonlinux").join(name));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/fiveplayonlinux").join(name))
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is set".to_owned())
}

fn load_saved_url() -> Option<String> {
    load_saved_value("launch-address")
}

fn load_saved_value(name: &str) -> Option<String> {
    let path = config_file(name).ok()?;
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn save_url(url: &str) -> Result<(), String> {
    save_config_value("launch-address", url)
}

fn save_launch_preferences(options: &Options) -> Result<(), String> {
    let performance = match options.performance {
        PerformanceMode::Balanced => "balanced",
        PerformanceMode::Performance => "performance",
        PerformanceMode::MemorySaver => "memory-saver",
    };
    save_config_value("performance", performance)?;
    save_config_value(
        "fullscreen",
        if options.fullscreen { "true" } else { "false" },
    )?;
    if let Some(gpu) = &options.selected_gpu {
        save_config_value("graphics-device", gpu)?;
    }
    Ok(())
}

fn save_config_value(name: &str, value: &str) -> Result<(), String> {
    let path = config_file(name)?;
    let parent = path
        .parent()
        .ok_or_else(|| "invalid configuration path".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create local configuration: {error}"))?;
    fs::write(&path, format!("{value}\n"))
        .map_err(|error| format!("cannot save local configuration: {error}"))
}

fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("Enter your Play V2 Fiveplay launch address first.".to_owned());
    }
    if url.starts_with("https://") || url.starts_with("http://localhost") {
        Ok(())
    } else {
        Err("The launch address must use HTTPS (or localhost for development).".to_owned())
    }
}

fn find_browser() -> Option<PathBuf> {
    [
        "microsoft-edge-stable",
        "chromium",
        "brave-origin",
        "brave-browser",
        "google-chrome-stable",
    ]
    .iter()
    .find_map(|name| resolve_executable(Path::new(name)))
}

fn resolve_executable(candidate: &Path) -> Option<PathBuf> {
    if candidate.components().count() > 1 {
        return is_executable(candidate).then(|| candidate.to_path_buf());
    }
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join(candidate))
            .find(|path| is_executable(path))
    })
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn print_command(command: &Command) {
    for (key, value) in command.get_envs() {
        if let Some(value) = value {
            print!("{}={} ", shell_quote(key), shell_quote(value));
        }
    }
    print!("{}", shell_quote(command.get_program()));
    for arg in command.get_args() {
        print!(" {}", shell_quote(arg));
    }
    println!();
}

fn shell_quote(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-._/:=*,~".contains(character))
    {
        value.into_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_when_needed() {
        assert_eq!(shell_quote(OsStr::new("plain-value_1")), "plain-value_1");
        assert_eq!(shell_quote(OsStr::new("two words")), "'two words'");
        assert_eq!(shell_quote(OsStr::new("it's")), "'it'\\''s'");
    }

    #[test]
    fn nvidia_prime_environment_is_complete() {
        let mut command = Command::new("browser");
        let selected_gpu = GpuInfo {
            name: "NVIDIA test device".to_owned(),
            kind: GpuKind::Discrete,
            vendor_id: 0x10de,
            device_id: 0x174d,
            api_version: ash::vk::API_VERSION_1_3,
            device_local_memory: 2 * 1024 * 1024 * 1024,
            bc_compression: true,
            graphics_queue: true,
            max_texture_size: 16384,
        };
        configure_gpu(&mut command, &selected_gpu);
        let variables: Vec<_> = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(OsStr::to_owned)))
            .collect();
        for (key, value) in [
            ("__GLX_VENDOR_LIBRARY_NAME", "nvidia"),
            ("__NV_PRIME_RENDER_OFFLOAD", "1"),
            ("__VK_LAYER_NV_optimus", "NVIDIA_only"),
            ("VK_LOADER_DRIVERS_SELECT", "*nvidia*"),
            ("DRI_PRIME", "1"),
        ] {
            assert!(variables.iter().any(|(actual_key, actual_value)| {
                actual_key == OsStr::new(key) && actual_value.as_deref() == Some(OsStr::new(value))
            }));
        }
    }

    #[test]
    fn ad_blocker_is_enabled_by_default() {
        let mut command = Command::new("browser");
        let options = Options {
            ad_block: true,
            browser: None,
            direct_launch: false,
            dry_run: false,
            fullscreen: false,
            graphics_check: false,
            integrated_gpu: false,
            performance: PerformanceMode::Balanced,
            selected_gpu: None,
            url: "https://example.invalid/launcher".to_owned(),
        };
        add_browser_args(&mut command, Path::new("/tmp/profile"), &options);
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        let rules = args
            .iter()
            .find(|arg| arg.starts_with("--host-resolver-rules="))
            .expect("ad-block resolver rules missing");
        assert!(rules.contains("quge5.com"));
        assert!(rules.contains("alwingulla.com"));
    }

    #[test]
    fn performance_and_fullscreen_flags_are_applied() {
        let mut command = Command::new("browser");
        let options = Options {
            ad_block: false,
            browser: None,
            direct_launch: false,
            dry_run: false,
            fullscreen: true,
            graphics_check: false,
            integrated_gpu: false,
            performance: PerformanceMode::Performance,
            selected_gpu: None,
            url: "https://example.invalid/launcher".to_owned(),
        };
        add_browser_args(&mut command, Path::new("/tmp/profile"), &options);
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        for expected in [
            "--start-fullscreen",
            "--disable-background-timer-throttling",
            "--disable-renderer-backgrounding",
            "--renderer-process-limit=8",
        ] {
            assert!(args.iter().any(|arg| arg == expected));
        }
    }
}
