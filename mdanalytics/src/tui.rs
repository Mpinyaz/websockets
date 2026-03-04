use chrono::{DateTime, Local, Utc};
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use mdanalytics::types::ActiveScreen;
use mdanalytics::types::AppMessage;
use mdanalytics::types::ManageAction;
use mdanalytics::types::TuiApp;
use mdanalytics::types::WsResponse;
use mdanalytics::types::{AssetClass, ConfirmationPayload, MarketUpdate, Payload};
use ratatui::{
    prelude::*,
    style::Stylize,
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
        TitlePosition, Wrap,
    },
};
use std::{collections::HashMap, io::stdout};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    init_terminal()?;

    let mut app = TuiApp::new();
    // Create channel for outgoing websocket messages
    let (ws_out_tx_main, mut ws_out_rx) = mpsc::unbounded_channel::<String>();
    app.ws_out_tx = Some(ws_out_tx_main);

    // Update channel to send AppMessage variants
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();
    let tx_websocket_handler = tx.clone();

    let url = std::env::var("WEBSOCKET_URL").expect("WEBSOCKET_URL must be set in .env");

    tokio::spawn(async move {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws_stream.split(); // Split stream for read/write

        loop {
            tokio::select! {
                // Handle incoming messages
                Some(Ok(msg)) = futures_util::StreamExt::next(&mut read) => {
                    if let Ok(text) = msg.to_text() {
                        if text.is_empty() {
                            continue; // Skip empty messages
                        }
                        if let Ok(data) = serde_json::from_str::<MarketUpdate>(text) {
                            let _ = tx_websocket_handler.send(AppMessage::MarketData(data));
                        } else if let Ok(conf_resp) = serde_json::from_str::<WsResponse>(text) {
                            let _ = tx_websocket_handler.send(AppMessage::WsConfirmation(conf_resp));
                        } else {
                            let unhandled_message = format!("Failed to parse WS message: {:?}", text);
                            // Try to parse as WsResponse to get more specific error
                            if let Err(e) = serde_json::from_str::<WsResponse>(text) {
                                eprintln!("Failed to parse as WsResponse: {} for text: {}", e, text);
                                let _ = tx_websocket_handler.send(AppMessage::WsConfirmation(WsResponse {
                                    message_type: "error".to_string(),
                                    payload: ConfirmationPayload {
                                        asset: "N/A".to_string(),
                                        status: format!("Parsing error: {}", e),
                                        symbol: vec![unhandled_message],
                                    },
                                }));
                            } else {
                                // Fallback for genuinely unhandled messages (shouldn't be reached if the above catches all errors)
                                let _ = tx_websocket_handler.send(AppMessage::WsConfirmation(WsResponse {
                                    message_type: "error".to_string(),
                                    payload: ConfirmationPayload {
                                        asset: "N/A".to_string(),
                                        status: "Unhandled".to_string(),
                                        symbol: vec![unhandled_message],
                                    },
                                }));
                            }
                        }
                    }
                }
                // Handle outgoing messages
                Some(outgoing_msg) = ws_out_rx.recv() => {
                    let _ = write.send(tokio_tungstenite::tungstenite::Message::Text(outgoing_msg.into())).await;
                }
                else => break, // Both streams are done
            }
        }
    });

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut event_stream = event::EventStream::new();

    while app.running {
        terminal.draw(|f| ui(f, &app))?;

        tokio::select! {
            Some(app_msg) = rx.recv() => {
                match app_msg {
                    AppMessage::MarketData(data) => app.update_market_data(data),
                    AppMessage::WsConfirmation(conf_resp) => {
                        // Handle adding/removing ticker based on confirmation
                        if let Some(_) = app.pending_manage_action.take() {
                            let response_asset_class_enum = conf_resp.payload.asset.parse::<AssetClass>();
                            if let Ok(asset_class_enum) = response_asset_class_enum {
                                if let Some(ticker_from_response) = conf_resp.payload.symbol.get(0) {
                                    if conf_resp.payload.status == "subscribed" {
                                        app.set_ws_status(format!("Successfully subscribed to {}", ticker_from_response), Color::Green, tx.clone());
                                    } else if conf_resp.payload.status == "unsubscribed" {
                                        match asset_class_enum {
                                            AssetClass::Crypto => app.crypto.remove(ticker_from_response),
                                            AssetClass::Forex => app.forex.remove(ticker_from_response),
                                            AssetClass::Equity => app.equity.remove(ticker_from_response),
                                        };
                                        app.set_ws_status(format!("Successfully unsubscribed from {}", ticker_from_response), Color::Green, tx.clone());
                                    } else {
                                        app.set_ws_status(format!("Unhandled status: {} for {}", conf_resp.payload.status, ticker_from_response), Color::Yellow, tx.clone());
                                    }
                                } else {
                                    app.set_ws_status(format!("WS Confirmation: Missing ticker symbol"), Color::Red, tx.clone());
                                }
                            } else {
                                app.set_ws_status(format!("WS Confirmation: Unknown asset class: {}", conf_resp.payload.asset), Color::Red, tx.clone());
                            }
                        } else {
                            app.set_ws_status(format!("WS Confirmation: No pending action for '{}'", conf_resp.payload.symbol.join(", ")), Color::Yellow, tx.clone());
                        }
                    },
                    AppMessage::ClearStatus => {
                        app.ws_status_message = String::from("Ready");
                        app.ws_status_color = Color::Gray;
                    }
                }
            }
            Some(Ok(Event::Key(key))) = futures_util::StreamExt::next(&mut event_stream) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => app.running = false,
                        KeyCode::Tab => app.next_screen(),
                        KeyCode::BackTab => app.prev_screen(),
                        _ => {
                            if app.show_popup { // Handle Manage screen popup dismissal
                                match key.code {
                                    KeyCode::Enter => app.confirm_manage_action(),
                                    KeyCode::Esc => app.cancel_manage_action(),
                                    _ => {}
                                }
                            } else if app.active_screen == ActiveScreen::Manage {
                                match key.code {
                                    KeyCode::Up => app.select_prev_manage_ticker(),
                                    KeyCode::Down => app.select_next_manage_ticker(),
                                    KeyCode::Right | KeyCode::Char('l') => {
                                        app.manage_selected_asset_class =
                                            match app.manage_selected_asset_class {
                                                AssetClass::Crypto => AssetClass::Forex,
                                                AssetClass::Forex => AssetClass::Equity,
                                                AssetClass::Equity => AssetClass::Crypto,
                                            };
                                        app.manage_selected_ticker_index = 0; // Reset selection
                                    },
                                    KeyCode::Left | KeyCode::Char('h') => {
                                        app.manage_selected_asset_class =
                                            match app.manage_selected_asset_class {
                                                AssetClass::Crypto => AssetClass::Equity,
                                                AssetClass::Equity => AssetClass::Forex,
                                                AssetClass::Forex => AssetClass::Crypto,
                                            };
                                        app.manage_selected_ticker_index = 0; // Reset selection
                                    },
                                    KeyCode::Char('s') => app.trigger_subscribe_popup(), // Trigger subscribe popup
                                    KeyCode::Char('u') => app.trigger_unsubscribe_popup(), // Trigger unsubscribe popup
                                    _ => {}
                                }
                            } else { // Handle Dashboard screen navigation
                                match key.code {
                                    KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
                                    KeyCode::Left | KeyCode::Char('h')  => app.prev_tab(),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

fn init_terminal() -> Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(frame: &mut Frame, app: &TuiApp) {
    let overall_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Global Banner
            Constraint::Min(0),    // Rest of the screen
            Constraint::Length(1), // Status Bar
        ])
        .split(frame.area());

    let banner_area = overall_layout[0];
    let main_area = overall_layout[1];
    let status_area = overall_layout[2];

    // Render global banner
    let banner = Paragraph::new("MDWS".slow_blink())
        .bold()
        .red()
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(banner, banner_area);

    // Split main_area into sidebar and content
    let main_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Sidebar width
            Constraint::Min(0),     // Content area
        ])
        .split(main_area);

    let sidebar_area = main_split[0];
    let screen_content_area = main_split[1];

    render_sidebar(frame, sidebar_area, app);

    match app.active_screen {
        ActiveScreen::Dashboard => render_dashboard_screen(frame, screen_content_area, app),
        ActiveScreen::Manage => render_manage_screen(frame, screen_content_area, app),
    }

    render_status_bar(frame, status_area, app);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let status = Paragraph::new(app.ws_status_message.clone())
        .style(Style::default().fg(app.ws_status_color))
        .alignment(Alignment::Left);
    frame.render_widget(status, area);
}

fn render_dashboard_screen(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let dashboard_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Market data table
        ])
        .split(area);

    let titles = ["Forex", "Crypto", "Equity"];
    let selected_index = match app.selected_tab {
        AssetClass::Forex => 0,
        AssetClass::Crypto => 1,
        AssetClass::Equity => 2,
    };

    let tabs = Tabs::new(titles)
        .select(selected_index)
        .style(Style::default().green())
        .highlight_style(Style::default().bold().blue());

    frame.render_widget(tabs, dashboard_layout[0]);

    match app.selected_tab {
        AssetClass::Forex => {
            render_market_table(frame, dashboard_layout[1], &app.forex, &app.selected_tab)
        }
        AssetClass::Crypto => {
            render_market_table(frame, dashboard_layout[1], &app.crypto, &app.selected_tab)
        }
        AssetClass::Equity => {
            render_market_table(frame, dashboard_layout[1], &app.equity, &app.selected_tab)
        }
    }
}

fn render_manage_screen(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let manage_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Asset Class Tabs
            Constraint::Min(0),    // Content Area
        ])
        .split(area);

    // Render Asset Class Tabs
    let asset_class_titles = ["Crypto", "Forex", "Equity"];
    let selected_asset_class_index = match app.manage_selected_asset_class {
        AssetClass::Crypto => 0,
        AssetClass::Forex => 1,
        AssetClass::Equity => 2,
    };
    let asset_class_tabs = Tabs::new(asset_class_titles)
        .select(selected_asset_class_index)
        .style(Style::default().green())
        .highlight_style(Style::default().bold().yellow())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Asset Classes")
                .title_position(TitlePosition::Top),
        );
    frame.render_widget(asset_class_tabs, manage_layout[0]);

    // Render Ticker List
    let empty_tickers = vec![]; // Longer-lived empty Vec<String>
    let current_tickers = app
        .tickers
        .get(&app.manage_selected_asset_class)
        .unwrap_or(&empty_tickers);
    let items: Vec<ListItem> = current_tickers
        .iter()
        .map(|s| ListItem::new(s.clone()))
        .collect();

    let manage_legend =
        "<Up/Down>: Select,<s>: Subscribe,<u>: Unsubscribe,<h,l>: Change Asset Class";

    let list_title = format!(
        "Available {} Tickers",
        app.manage_selected_asset_class.to_string()
    );
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(list_title)
                .title_position(TitlePosition::Top)
                .title(manage_legend)
                .title_position(TitlePosition::Bottom),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    let mut state = ListState::default();
    if !current_tickers.is_empty() {
        state.select(Some(app.manage_selected_ticker_index));
    }

    frame.render_stateful_widget(list, manage_layout[1], &mut state);

    // Render Popup if active
    if app.show_popup {
        let popup_text = if let Some(action) = &app.popup_action {
            let ticker = current_tickers
                .get(app.manage_selected_ticker_index)
                .map(|s| s.as_str()) // Convert &String to &str
                .unwrap_or("Unknown"); // Use a static string literal
            match action {
                ManageAction::Subscribe => {
                    format!("Confirm subscription to {}?", ticker)
                }
                ManageAction::Unsubscribe => {
                    format!("Confirm unsubscription from {}?", ticker)
                }
            }
        } else {
            String::from("Confirm action?")
        };

        let popup_block = Block::default()
            .title("Confirmation")
            .title_position(TitlePosition::Top)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Red));

        let area = centered_rect(60, 25, area); // Increased height to 25%

        let inner_area = popup_block.inner(area);

        // Split inner area vertically: Message (top) and Buttons (bottom)
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Message space
                Constraint::Length(1), // Button row
            ])
            .margin(1)
            .split(inner_area);

        let popup_paragraph = Paragraph::new(popup_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        // Split bottom row horizontally for Yes/No buttons
        let popup_button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(popup_layout[1]);

        let yes_button = Paragraph::new("Yes (Enter)")
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));
        let no_button = Paragraph::new("No (Esc)")
            .alignment(Alignment::Center)
            .style(Style::default().bg(Color::DarkGray));

        frame.render_widget(Clear, area); // Clear the background
        frame.render_widget(popup_block, area);
        frame.render_widget(popup_paragraph, popup_layout[0]);
        frame.render_widget(yes_button, popup_button_layout[0]);
        frame.render_widget(no_button, popup_button_layout[1]);
    }
}

// Helper function to create a centered rectangle for popups
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_sidebar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let titles = ["Dashboard", "Manage"];
    let items: Vec<ListItem> = titles.iter().map(|&s| ListItem::new(s)).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("<Tab>/<S-Tab>")
                .title_position(TitlePosition::Bottom),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )
        .highlight_symbol(">> ");

    let selected_index = match app.active_screen {
        ActiveScreen::Dashboard => 0,
        ActiveScreen::Manage => 1,
    };

    let mut state = ListState::default();
    state.select(Some(selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}

fn render_market_table(
    frame: &mut Frame,
    area: Rect,
    data: &HashMap<String, MarketUpdate>,
    asset_class: &AssetClass,
) {
    let (header, constraints, rows): (Row, Vec<Constraint>, Vec<Row>) = match asset_class {
        AssetClass::Forex => {
            let header = Row::new(vec![
                "Ticker",
                "Bid Price",
                "Mid Price",
                "Ask Price",
                "Time",
            ])
            .style(Style::default().bold())
            .height(1);
            let constraints = vec![
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ];
            let rows = data
                .iter()
                .map(|(ticker, update)| {
                    if let Payload::Forex {
                        mid_price,
                        timestamp,
                        bid_price,
                        ask_price,
                        ..
                    } = &update.payload
                    {
                        Row::new(vec![
                            ticker.clone(),
                            bid_price.to_string(),
                            mid_price.to_string(),
                            ask_price.to_string(),
                            timestamp
                                .parse::<DateTime<Utc>>()
                                .map(|dt| {
                                    dt.with_timezone(&Local)
                                        .format("%d/%m/%y %H:%M")
                                        .to_string()
                                })
                                .unwrap_or_else(|_| timestamp.clone()),
                        ])
                    } else {
                        Row::new(vec![
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                        ])
                    }
                })
                .collect();
            (header, constraints, rows)
        }
        AssetClass::Crypto => {
            let header = Row::new(vec!["Ticker", "Type", "Price", "Size", "Exchange", "Time"])
                .style(Style::default().bold())
                .height(1);
            let constraints = vec![
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ];
            let rows = data
                .iter()
                .map(|(ticker, update)| {
                    if let Payload::Crypto {
                        last_price,
                        date,
                        last_size,
                        exchange,
                        update_type,
                        ..
                    } = &update.payload
                    {
                        Row::new(vec![
                            ticker.clone(),
                            if update_type == "T" { "Trade" } else { "Quote" }.to_string(),
                            last_price.to_string(),
                            last_size.to_string(),
                            exchange.clone(),
                            date.parse::<DateTime<Utc>>()
                                .map(|dt| {
                                    dt.with_timezone(&Local)
                                        .format("%d/%m/%y %H:%M")
                                        .to_string()
                                })
                                .unwrap_or_else(|_| date.clone()),
                        ])
                    } else {
                        Row::new(vec![
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                        ])
                    }
                })
                .collect();
            (header, constraints, rows)
        }
        AssetClass::Equity => {
            let header = Row::new(vec!["Ticker", "Type", "Price", "Vol/Size", "Time"])
                .style(Style::default().bold())
                .height(1);
            let constraints = vec![
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ];
            let rows = data
                .iter()
                .map(|(ticker, update)| {
                    if let Payload::Equity {
                        update_type,
                        last_price,
                        mid_price,
                        last_size,
                        volume,
                        date,
                        ..
                    } = &update.payload
                    {
                        let price = last_price.or(*mid_price).unwrap_or(0.0);
                        let vol = last_size.or(*volume).unwrap_or(0.0);
                        Row::new(vec![
                            ticker.clone(),
                            match update_type.as_str() {
                                "T" => "Trade",
                                "Q" => "Quote",
                                "B" => "Bar",
                                _ => update_type,
                            }.to_string(),
                            price.to_string(),
                            vol.to_string(),
                            date.parse::<DateTime<Utc>>()
                                .map(|dt| {
                                    dt.with_timezone(&Local)
                                        .format("%d/%m/%y %H:%M")
                                        .to_string()
                                })
                                .unwrap_or_else(|_| date.clone()),
                        ])
                    } else {
                        Row::new(vec![
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                            String::from(""),
                        ])
                    }
                })
                .collect();
            (header, constraints, rows)
        }
    };

    let legend_text = " Use <q> to quit, <h,l> or <←,→> to navigate tabs ";

    let table = Table::new(rows, &constraints).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Market Data")
            .title_position(TitlePosition::Bottom)
            .title(legend_text),
    );

    frame.render_widget(table, area);
}
