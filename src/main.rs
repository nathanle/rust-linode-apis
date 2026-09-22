use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::event::{self, Event, KeyCode};
use std::io::{stdout, Write};

const API_VERSION: &str = "v4";

// --- DATA MODELS ---
#[derive(Serialize, Deserialize, Debug, Clone)]
struct LinodeVMObject {
    alerts: LinodeObject,
    backups: BackUps,
    #[serde(default)]
    capabilities: Vec<String>,
    disk_encryption: String,
    group: String,
    has_user_data: bool,
    host_uuid: String,
    hypervisor: String,
    id: i32,
    image: String,
    interface_generation: String,
    ipv4: Vec<String>,
    ipv6: String,
    label: String,
    lke_cluster_id: Option<i32>, // Changed from String to i32 to match the JSON payload
    maintenance_policy: String,
    placement_group: Option<String>,
    region: String,
    site_type: String,
    specs: Specs,
    status: String,
    tags: Vec<String>,
    r#type: String,
    updated: String,
    watchdog_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BackUps {
    available: bool,
    enabled: bool,
    last_successful: Option<String>,
    schedule: Schedule,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Schedule {
    day: Option<String>,
    window: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LinodeObject {
    cpu: i32,
    io: i32,
    network_in: i32,
    network_out: i32,
    transfer_quota: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Specs {
    accelerated_devices: i32,
    disk: i32,
    gpus: i32,
    memory: i32,
    transfer: i32,
    vcpus: i32,
}

#[derive(Deserialize, Debug)]
struct LinodeResponse {
    data: Vec<LinodeVMObject>,
}


async fn fetch_linodes(api_key: &str) -> Vec<LinodeVMObject> {
    let client = reqwest::Client::new();
    let url = format!("https://api.linode.com/{API_VERSION}/linode/instances");

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await;

    match response {
        Ok(res) => {
            // Get the body as a raw string first
            let body = res.text().await.unwrap_or_default();
            println!("RAW JSON: {}", body); // <--- THIS WILL SHOW US THE PROBLEM

            // Try to parse it into a generic Value first to see if it's even valid JSON
            let obj: serde_json::Value = serde_json::from_str(&body).expect("Invalid JSON format");
            
            // Now try to extract the data array and map it to our structs
            if let Some(data_array) = obj["data"].as_array() {
                let mut vms = Vec::new();
                for item in data_array {
                    // Use from_value on each individual item to skip fields we don't care about
                    if let Ok(vm) = serde_json::from_value::<LinodeVMObject>(item.clone()) {
                        vms.push(vm);
                    }
                }
                vms
            } else {
                println!("No 'data' array found in JSON");
                vec![]
            }
        }
        Err(e) => {
            eprintln!("Request failed: {}", e);
            vec![]
        }
    }
}


// --- TUI STATE ---
struct App {
    vms: Vec<LinodeVMObject>,
    selected_index: usize,
    show_details: bool,
}

impl App {
    fn new(vms: Vec<LinodeVMObject>) -> Self {
        Self {
            vms,
            selected_index: 0,
            show_details: false,
        }
    }

    fn next(&mut self) {
        if self.selected_index < self.vms.len() - 1 {
            self.selected_index += 1;
        }
    }

    fn prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
}

// --- UI RENDERING ---
fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(f.size());

    let header = Paragraph::new("Linode Manager - Arrows to navigate, Enter for details, Esc to back, 'q' to quit")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    if app.show_details {
        if let Some(vm) = app.vms.get(app.selected_index) {
            let details = format!(
                "ID: {}\n\
                 Label: {}\n\
                 Status: {}\n\
                 Region: {}\n\
                 Memory: {} MB\n\
                 vCPUs: {}\n\
                 Type: {}\n\
                 Host UUID: {}\n\
                 <-- Press 'Esc' to return to list.",
                vm.id, 
                vm.label, 
                vm.status, 
                vm.region, 
                vm.specs.memory, // <--- Accessing via .specs
                vm.specs.vcpus,  // <--- Accessing via .specs
                vm.r#type,
                vm.host_uuid
            );
            let detail_widget = Paragraph::new(details)
                .block(Block::default().title("Linode Details").borders(Borders::ALL));
            f.render_widget(detail_widget, chunks[1]);
        }
    } else {
        let items: Vec<ListItem> = app.vms.iter().enumerate().map(|(i, vm)| {
            let style = if i == app.selected_index {
                Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("[{}] {} - {}", vm.id, vm.label, vm.status)).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().title("Your Linodes").borders(Borders::ALL));
        f.render_widget(list, chunks[1]);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Fetch Data
    let api_key = std::env::var("LINODE_RUST_PAT").expect("Set LINODE_RUST_PAT env var");
    println!("Fetching linodes...");
    let vms = fetch_linodes(&api_key).await;

    if vms.is_empty() {
        println!("No linodes found or error occurred.");
        return Ok(());
    }

    // 2. Setup Terminal
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. App State
    let mut app = App::new(vms);

    // 4. Main Loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Char('q') => break,
                KeyCode::Up => app.prev(),
                KeyCode::Down => app.next(),
                KeyCode::Enter => app.show_details = !app.show_details,
                KeyCode::Esc => app.show_details = false,
                _ => {}
            }
        }
    }

    // 5. Restore Terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}
