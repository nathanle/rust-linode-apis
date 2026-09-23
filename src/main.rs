use reqwest::header::HeaderName;
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
use std::collections::HashMap;

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

// --- MAP COORDINATES ---
// You can adjust these coordinates to "shape" the map as you like!
fn get_region_coords(region: &str) -> Option<(u16, u16)> {
    match region {
        "us-east" => Some((10, 15)),
        "us-southeast" => Some((15, 12)),
        "us-west" => Some((5, 10)),
        "us-ord" => Some((12, 18)),
        "eu-north" => Some((25, 15)),
        "ap-southeast" => Some((35, 10)),
        "ap-northeast" => Some((35, 20)),
        _ => Some((0, 0)), // Default for unknown regions
    }
}

// --- APP STATE ---
#[derive(PartialEq, Debug)]
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
        .split(f.size());

    let header = Paragraph::new(format!(
        "Linode Manager | Mode: {:?} | Press 'm' for Map, 'l' for List, 'd' for Details, 'q' to quit",
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
            // Aggregate counts by region
            let mut counts: HashMap<String, usize> = HashMap::new();
            for vm in &app.vms {
                *counts.entry(vm.region.clone()).or_insert(0) += 1;
            }

            // Create a grid buffer (40 columns wide, 20 rows high)
            let mut grid = vec![" ".to_string(); 40 * 20];

            // Draw a border
            for x in 0..40 {
                grid[x] = "|".to_string();             // Top
                grid[x + 40 * 19] = "|".to_string();   // Bottom
            }
            for y in 0..20 {
                grid[y] = "-".to_string();              // Left
                grid[y + 40 - 1] = "-".to_string();     // Right
            }

            // Plot the dots
            for (region, count) in counts {
                if let Some((x, y)) = get_region_coords(&region) {
                    // Adjusting for 0-indexed grid and flipping Y (since terminal rows go down)
                    let grid_x = x as usize;
                    let grid_y = (19 - y as usize) as usize;
                    
                    if grid_x < 40 && grid_y < 20 {
                        grid[grid_y * 40 + grid_x] = "•".to_string();
                        // Add count next to it
                        if grid_x + 2 < 40 {
                            grid[grid_y * 40 + grid_x + 2] = count.to_string();
                        }
                    }
                }
            }

            let map_str: String = grid.into_iter().collect();
            let map_widget = Paragraph::new(map_str)
                .block(Block::default().title("Geographic Distribution").borders(Borders::ALL));
            f.render_widget(map_widget, chunks[1]);
        }
    }
}

// --- FETCHING LOGIC ---
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
            let obj: serde_json::Value = serde_json::from_str(&body).expect("Invalid JSON");
            if let Some(data_array) = obj["data"].as_array() {
                data_array.iter()
                    .filter_map(|item| serde_json::from_value::<LinodeVMObject>(item.clone()).ok())
                    .collect()
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("LINODE_RUST_PAT").expect("Set LINODE_RUST_PAT env var");
    let vms = fetch_linodes(&api_key).await;

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
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

    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}
