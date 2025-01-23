use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde_yaml::Value;
use std::{
    env, io,
    io::Read,
    path::Path,
    time::Duration,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

#[derive(Debug, PartialEq)]
struct TreeNode {
    key: String,
    value: Value,
    children: Vec<TreeNode>,
    expanded: bool,
    visible: bool,
    depth: usize,
}

impl TreeNode {
    fn new(key: String, value: Value, depth: usize) -> Self {
        let mut children = vec![];
        let expanded = false;
        let visible = true;

        match &value {
            Value::Mapping(map) => {
                for (k, v) in map {
                    let key_str = serde_yaml::to_string(k).unwrap().trim().to_string();
                    children.push(TreeNode::new(key_str, v.clone(), depth + 1));
                }
            }
            Value::Sequence(seq) => {
                for (i, v) in seq.iter().enumerate() {
                    children.push(TreeNode::new(format!("[{}]", i), v.clone(), depth + 1));
                }
            }
            Value::Tagged(tagged) => {
                children.push(TreeNode::new("tagged".to_string(), tagged.value.clone(), depth + 1));
            }
            _ => {}
        }

        Self {
            key,
            value,
            children,
            expanded,
            visible,
            depth,
        }
    }

    fn flatten(&self) -> Vec<&TreeNode> {
        let mut result = vec![self];
        if self.expanded {
            for child in &self.children {
                if matches!(child.value, Value::Mapping(_) | Value::Sequence(_)) && child.expanded {
                    let mut child_nodes = child.flatten();
                    result.append(&mut child_nodes);
                } else {
                    result.push(child);
                }
            }
        }
        result
    }
}

fn build_yaml_tree(input: &str) -> TreeNode {
    let value: Value = serde_yaml::from_str(input).unwrap();
    TreeNode::new("root".to_string(), value, 0)
}

fn main() -> io::Result<()> {
    let mut input = String::new();
    let args: Vec<String> = env::args().collect();

    let mut debug_mode = false;
    let mut file_path = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                file_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--debug" => {
                debug_mode = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    if let Some(path) = file_path {
        input = std::fs::read_to_string(Path::new(&path))?;
    } else {
        io::stdin().read_to_string(&mut input)?;
    }

    let mut tree = build_yaml_tree(&input);
    tree.expanded = true;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut cursor_pos = 0;
    let mut should_quit = false;

    while !should_quit {
            terminal.draw(|f| {
                let size = f.size();
                let nodes = tree.flatten();
                let max_y = (size.height - 1) as usize;

                let chunks = Layout::default()
                    .direction(if debug_mode { Direction::Horizontal } else { Direction::Vertical })
                    .constraints(
                        if debug_mode {
                            [Constraint::Percentage(70), Constraint::Percentage(30)].as_ref()
                        } else {
                            [Constraint::Percentage(100)].as_ref()
                        }
                    )
                    .split(size);

                let text = nodes
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    let indent = "  ".repeat(node.depth);
                    let (prefix, content) = match &node.value {
                        Value::Mapping(_) => {
                            if node.children.is_empty() {
                                (" ", format!("{{}}"))
                            } else if node.expanded {
                                ("▼", "".to_string())
                            } else {
                                ("▶", format!("{{...}}"))
                            }
                        }
                        Value::Sequence(_) => {
                            if node.children.is_empty() {
                                (" ", "[]".to_string())
                            } else if node.expanded {
                                ("▼", "".to_string())
                            } else {
                                ("▶", "[...]".to_string())
                            }
                        }
                        Value::Tagged(tagged) => {
                            (" ", serde_yaml::to_string(&tagged.value).unwrap().trim().to_string())
                        }
                        _ => (" ", serde_yaml::to_string(&node.value).unwrap().trim().to_string()),
                    };

                    let full_text = if matches!(node.value, Value::Mapping(_) | Value::Sequence(_)) && node.expanded {
                        format!("{} {}{}:", prefix, indent, node.key)
                    } else {
                        format!("{} {}{}: {}", prefix, indent, node.key, content)
                    };
                    let max_width = (size.width - 2) as usize; // Account for borders
                    let wrapped_text = textwrap::wrap(&full_text, max_width)
                        .into_iter()
                        .map(|line| {
                            if i == cursor_pos {
                                Spans::from(vec![
                                    Span::styled(
                                        line.to_string(),
                                        Style::default()
                                            .fg(Color::Yellow)
                                            .add_modifier(Modifier::BOLD),
                                    )
                                ])
                            } else {
                                Spans::from(vec![Span::raw(line.to_string())])
                            }
                        })
                        .collect::<Vec<_>>();

                    // Add proper spacing between wrapped lines
                    let wrapped_len = wrapped_text.len();
                    let mut result = Vec::new();
                    for (line_idx, line) in wrapped_text.into_iter().enumerate() {
                        result.push(line);
                        // Add spacing between wrapped lines except the last one
                        if line_idx < wrapped_len - 1 {
                            result.push(Spans::from(vec![Span::raw("")]));
                        }
                    }
                    result
                })
                .flatten()
                .collect::<Vec<_>>();

            let main_panel = Paragraph::new(text).block(Block::default().borders(Borders::ALL));
            
            if debug_mode {
                let debug_info = if let Some(node) = nodes.get(cursor_pos) {
                    let mut path = vec![];
                    let mut current = node;
                    while let Some(parent) = nodes.iter().find(|n| n.children.contains(current)) {
                        path.push(parent.key.clone());
                        current = parent;
                    }
                    path.reverse();
                    path.push(node.key.clone());
                    let path = path.join(" > ");
                    
                    let value_type = match &node.value {
                        Value::Null => "Null",
                        Value::Bool(_) => "Bool",
                        Value::Number(_) => "Number",
                        Value::String(_) => "String",
                        Value::Sequence(_) => "Sequence",
                        Value::Mapping(_) => "Mapping",
                        Value::Tagged(_) => "Tagged",
                    };
                    
                    let expanded_status = if matches!(node.value, Value::Mapping(_) | Value::Sequence(_)) {
                        if node.expanded { "Expanded" } else { "Collapsed" }
                    } else {
                        "N/A"
                    };
                    
                    Text::from(vec![
                        Spans::from(vec![
                            Span::raw("Path: "),
                            Span::styled(path, Style::default().fg(Color::Cyan)),
                        ]),
                        Spans::from(vec![
                            Span::raw("Type: "),
                            Span::styled(value_type, Style::default().fg(Color::Green)),
                        ]),
                        Spans::from(vec![
                            Span::raw("Status: "),
                            Span::styled(expanded_status, Style::default().fg(Color::Yellow)),
                        ]),
                    ])
                } else {
                    Text::from("No node selected")
                };
                
                let debug_panel = Paragraph::new(debug_info)
                    .block(Block::default().borders(Borders::ALL).title("Debug Info"));
                
                f.render_widget(main_panel, chunks[0]);
                f.render_widget(debug_panel, chunks[1]);
            } else {
                f.render_widget(main_panel, size);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => should_quit = true,
                        KeyCode::Down | KeyCode::Char('j') => {
                            let total_lines = tree.flatten().len();
                            if cursor_pos < total_lines - 1 {
                                cursor_pos += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if cursor_pos > 0 {
                                cursor_pos -= 1;
                            }
                        }
                        KeyCode::Char('h') => {
                            // Get the current node
                            let nodes = tree.flatten();
                            if let Some(node) = nodes.get(cursor_pos) {
                                // Only process mappable/sequence nodes
                                if matches!(node.value, Value::Mapping(_) | Value::Sequence(_)) {
                                    // Build the full path to this node
                                    let mut path = vec![];
                                    let mut current = node;
                                    while let Some(parent) = nodes.iter().find(|n| n.children.contains(current)) {
                                        path.push(parent.key.clone());
                                        current = parent;
                                    }
                                    path.reverse();
                                    path.push(node.key.clone());
                                    
                                    // Traverse the tree using the path
                                    let mut current = &mut tree;
                                    for key in path.iter().skip(1) {
                                        if let Some(found) = current
                                            .children
                                            .iter_mut()
                                            .find(|child| &child.key == key)
                                        {
                                            current = found;
                                        } else {
                                            break;
                                        }
                                    }
                                    
                                    // Toggle the expanded state
                                    current.expanded = !current.expanded;
                                }
                            }
                        }
                        KeyCode::Char('l') => {
                            // Get the current node
                            let nodes = tree.flatten();
                            if let Some(node) = nodes.get(cursor_pos) {
                                // Only process mappable/sequence nodes
                                if matches!(node.value, Value::Mapping(_) | Value::Sequence(_)) {
                                    // Build the full path to this node
                                    let mut path = vec![];
                                    let mut current = node;
                                    while let Some(parent) = nodes.iter().find(|n| n.children.contains(current)) {
                                        path.push(parent.key.clone());
                                        current = parent;
                                    }
                                    path.reverse();
                                    path.push(node.key.clone());
                                    
                                    // Traverse the tree using the path
                                    let mut current = &mut tree;
                                    for key in path.iter().skip(1) {
                                        if let Some(found) = current
                                            .children
                                            .iter_mut()
                                            .find(|child| &child.key == key)
                                        {
                                            current = found;
                                        } else {
                                            break;
                                        }
                                    }
                                    
                                    // Toggle the expanded state
                                    current.expanded = !current.expanded;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
