use gtk::gdk;
use gtk::prelude::*;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const AD_BLOCK_RULES: &str = "MAP quge5.com ~NOTFOUND, MAP *.quge5.com ~NOTFOUND, MAP alwingulla.com ~NOTFOUND, MAP *.alwingulla.com ~NOTFOUND";

const CSS: &str = r#"
window { background: #080b10; color: #f4f7f8; }
headerbar { background: #080b10; border-bottom: 1px solid rgba(255,255,255,0.07); min-height: 52px; }
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
.settings-card { background: #0e1319; border: 1px solid rgba(255,255,255,0.07); border-radius: 18px; padding: 8px 18px; }
.setting-row { padding: 11px 2px; }
.setting-title { font-weight: 700; }
.setting-copy { color: #89959b; font-size: 12px; }
switch:checked { background: #58d993; }
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
    integrated_gpu: bool,
    url: String,
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
        integrated_gpu: false,
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
            Some("--integrated") => options.integrated_gpu = true,
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
           --no-adblock      Allow the site's advertising hosts\n\
           --browser PATH    Use a specific Chromium-family executable\n\
           --url URL         Use a secure Play V2 launch address\n\
           --integrated      Prefer the integrated GPU\n\
           --dry-run         Print the resolved launch command\n\
           -h, --help        Show this help\n",
        version = env!("CARGO_PKG_VERSION")
    );
}

fn game_command(browser: &Path, profile: &Path, options: &Options) -> Result<Command, String> {
    validate_url(&options.url)?;
    let mut command = Command::new(browser);
    add_browser_args(&mut command, profile, &options.url, options.ad_block);
    if !options.integrated_gpu {
        configure_nvidia_prime(&mut command);
    }
    Ok(command)
}

fn add_browser_args(command: &mut Command, profile: &Path, url: &str, ad_block: bool) {
    command
        .arg(format!("--app={url}"))
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
        .arg("--renderer-process-limit=6")
        .arg("--ozone-platform-hint=auto");
    if ad_block {
        command.arg(format!("--host-resolver-rules={AD_BLOCK_RULES}"));
    }
}

fn configure_nvidia_prime(command: &mut Command) {
    command
        .env("__GLX_VENDOR_LIBRARY_NAME", "nvidia")
        .env("__NV_PRIME_RENDER_OFFLOAD", "1")
        .env("__VK_LAYER_NV_optimus", "NVIDIA_only")
        .env("VK_LOADER_DRIVERS_SELECT", "*nvidia*");
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
        .default_width(820)
        .default_height(680)
        .resizable(true)
        .build();
    let header = gtk::HeaderBar::new();
    let header_title = gtk::Label::new(Some("PLAY V2 FIVEPLAY"));
    header_title.add_css_class("app-title");
    header.set_title_widget(Some(&header_title));
    window.set_titlebar(Some(&header));

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
        "A clean WebGPU session tuned for your NVIDIA MX130.",
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
        "video-display-symbolic",
        "Dedicated graphics",
        "NVIDIA PRIME offload",
    ));
    features.append(&feature_card(
        "security-high-symbolic",
        "Clean session",
        "Ad hosts blocked early",
    ));
    features.append(&feature_card(
        "applications-engineering-symbolic",
        "WebGPU ready",
        "Acceleration enforced",
    ));
    shell.append(&features);

    let settings = gtk::Box::new(gtk::Orientation::Vertical, 0);
    settings.add_css_class("settings-card");
    let address_entry = gtk::Entry::builder()
        .text(&options.url)
        .placeholder_text("Paste your secure launch address")
        .hexpand(false)
        .width_chars(32)
        .build();
    let gpu_switch = gtk::Switch::builder()
        .active(!options.integrated_gpu)
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
    settings.append(&setting_row(
        "Use NVIDIA MX130",
        "Routes Vulkan and WebGPU through the dedicated graphics card.",
        &gpu_switch,
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
    play.connect_clicked(move |_| {
        let launch_options = Options {
            ad_block: block_switch.is_active(),
            integrated_gpu: !gpu_switch.is_active(),
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

fn label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_halign(gtk::Align::Start);
    label.set_wrap(true);
    label.add_css_class(class);
    label
}

fn feature_card(icon_name: &str, title: &str, copy: &str) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 7);
    card.add_css_class("feature-card");
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(22);
    icon.set_halign(gtk::Align::Start);
    icon.add_css_class("feature-icon");
    card.append(&icon);
    card.append(&label(title, "feature-title"));
    card.append(&label(copy, "feature-copy"));
    card
}

fn setting_row(title: &str, copy: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("setting-row");
    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    text.append(&label(title, "setting-title"));
    text.append(&label(copy, "setting-copy"));
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

fn config_file() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(path).join("fiveplayonlinux/launch-address"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/fiveplayonlinux/launch-address"))
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is set".to_owned())
}

fn load_saved_url() -> Option<String> {
    let path = config_file().ok()?;
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn save_url(url: &str) -> Result<(), String> {
    let path = config_file()?;
    let parent = path
        .parent()
        .ok_or_else(|| "invalid configuration path".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create local configuration: {error}"))?;
    fs::write(&path, format!("{url}\n"))
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
        configure_nvidia_prime(&mut command);
        let variables: Vec<_> = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(OsStr::to_owned)))
            .collect();
        for (key, value) in [
            ("__GLX_VENDOR_LIBRARY_NAME", "nvidia"),
            ("__NV_PRIME_RENDER_OFFLOAD", "1"),
            ("__VK_LAYER_NV_optimus", "NVIDIA_only"),
            ("VK_LOADER_DRIVERS_SELECT", "*nvidia*"),
        ] {
            assert!(variables.iter().any(|(actual_key, actual_value)| {
                actual_key == OsStr::new(key) && actual_value.as_deref() == Some(OsStr::new(value))
            }));
        }
    }

    #[test]
    fn ad_blocker_is_enabled_by_default() {
        let mut command = Command::new("browser");
        add_browser_args(
            &mut command,
            Path::new("/tmp/profile"),
            "https://example.invalid/launcher",
            true,
        );
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
}
