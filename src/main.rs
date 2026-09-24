use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Terminal;
use ratatui::widgets::canvas::{Canvas, Map, MapResolution, Points, Rectangle};

use serde::{Deserialize, Serialize};
use std::collections::HashMap; // Added missing import
use std::io::stdout;           // Added missing import

const API_VERSION: &str = "v4";

// --- DATA MODELS ---

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LinodeVMObject {
    id: i32,
    label: String,
    status: String,
    region: String,
    specs: Specs,
    lke_cluster_id: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Specs {
    memory: i32,
    vcpus: i32,
}

#[derive(Deserialize, Debug)]
struct LinodeResponse {
    data: Vec<LinodeVMObject>,
}

// --- DATA FETCHING ---

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
            let body = res.text().await.unwrap_or_default();
            let obj: serde_json::Value = serde_json::from_str(&body).expect("Invalid JSON format");
            if let Some(data_array) = obj["data"].as_array() {
                data_array.iter()
                    .filter_map(|item| serde_json::from_value::<LinodeVMObject>(item.clone()).ok())
                    .collect()
            } else {
                vec![]
            }
        }
        Err(e) => {
            eprintln!("Request failed: {}", e);
            vec![]
        }
    }
}

// Helper to map region names to coordinates
fn get_coords_for_region(region: &str) -> (f64, f64) {
    match region {
        "us-east" => (-75.0, 38.0),
        "us-southeast" => (-81.0, 33.0),
        "us-ord" => (-87.0, 41.0),
        "us-west" => (-120.0, 37.0),
        "eu-north" => (10.0, 50.0),
        "ap-southeast" => (118.0, -3.0),
        "ap-northeast" => (139.0, 35.0),
        _ => (0.0, 0.0),
    }
}

// --- TUI STATE ---

#[derive(Debug, PartialEq)]
enum ViewMode {
    List,
    Detail,
    Map,
}

struct App {
    vms: Vec<LinodeVMObject>,
    selected_index: usize,
    view_mode: ViewMode,
}

impl App {
    fn new(vms: Vec<LinodeVMObject>) -> Self {
        Self {
            vms,
            selected_index: 0,
            view_mode: ViewMode::List,
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
        .split(f.area());

    let header = Paragraph::new(format!(
        "Linode Manager | Mode: {:?} | [m]ap [l]ist [d]etail [q]uit",
        app.view_mode
    ))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    match app.view_mode {
        ViewMode::List => {
            let items: Vec<ListItem> = app.vms.iter().enumerate().map(|(i, vm)| {
                let style = if i == app.selected_index {
                    Style::default().fg(Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("[{}] {} - {}", vm.id, vm.label, vm.status)).style(style)
            }).collect();

            let list = List::new(items).block(Block::default().title("Your Linodes").borders(Borders::ALL));
            f.render_widget(list, chunks[1]);
        }
        ViewMode::Detail => {
            if let Some(vm) = app.vms.get(app.selected_index) {
                let details = format!(
                    "ID: {}\n\
                 Label: {}\n\
                 Status: {}\n\
                 Region: {}\n\
                 Memory: {} MB\n\
                 vCPUs: {}\n\
                 LKE ID: {}\n\
                 <-- Press 'd' to go back to list.",
                vm.id, vm.label, vm.status, vm.region, vm.specs.memory, vm.specs.vcpus, vm.lke_cluster_id.unwrap_or(0)
                );
                let detail_widget = Paragraph::new(details).block(Block::default().title("Linode Details").borders(Borders::ALL));
                f.render_widget(detail_widget, chunks[1]);
            }
        }
        ViewMode::Map => {
            let mut counts: HashMap<String, usize> = HashMap::new();
            for vm in &app.vms {
                *counts.entry(vm.region.clone()).or_insert(0) += 1;
            }

            let canvas = Canvas::default()
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0])
                .paint(|ctx| {
                    ctx.draw(&Map {
                        resolution: MapResolution::High,
                        color: Color::White,
                    });

                    let mut points = Vec::new();
                    for vm in &app.vms {
                        let (lon, lat) = get_coords_for_region(&vm.region);
                        points.push((lon, lat));
                    }

                    // Fixed: Borrow the points vector here
                    ctx.draw(&Points {
                        coords: &points,
                        color: Color::Yellow,
                    });

                    if let Some(vm) = app.vms.get(app.selected_index) {
                        let (lon, lat) = get_coords_for_region(&vm.region);
                        ctx.draw(&Rectangle {
                            x: lon + 2.0,
                            y: lat - 1.0,
                            width: 4.0,
                            height: 1.0,
                            color: Color::Cyan,
                        });
                    }
                });

            f.render_widget(canvas, chunks[1]);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("LINODE_RUST_PAT").expect("Set LINODE_RUST_PAT env var");
    
    println!("Fetching linodes...");
    let vms = fetch_linodes(&api_key).await;

    if vms.is_empty() {
        println!("No linodes found or error occurred.");
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(vms);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('m') => app.view_mode = ViewMode::Map,
                KeyCode::Char('l') => app.view_mode = ViewMode::List,
                KeyCode::Char('d') => app.view_mode = ViewMode::Detail,
                KeyCode::Up => app.prev(),
                KeyCode::Down => app.next(),
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
