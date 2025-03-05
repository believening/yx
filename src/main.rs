use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use serde_yaml::Value;
use std::{env, io, io::Read, path::Path, time::Duration};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
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
                children.push(TreeNode::new(
                    "tagged".to_string(),
                    tagged.value.clone(),
                    depth + 1,
                ));
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

fn process_node(tree: &mut TreeNode, cursor_pos: usize, mut action: impl FnMut(&mut TreeNode)) {
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

            // Traverse the tree using the path and collect mutable references
            let mut target_node: Option<&mut TreeNode> = Some(tree);
            for key in path.iter().skip(1) {
                if let Some(current_mut) = target_node {
                    if let Some(found) = current_mut
                        .children
                        .iter_mut()
                        .find(|child| &child.key == key)
                    {
                        target_node = Some(found);
                    } else {
                        target_node = None;
                        break;
                    }
                }
            }

            // Apply action to the found node
            if let Some(target) = target_node {
                action(target);
            }
        }
    }
}

// 定义应用状态枚举
#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Normal,
    Search,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut debug_mode = false;
    let mut show_help = false;
    let mut file_path = None;

    // 解析命令行参数
    for i in 1..args.len() {
        match args[i].as_str() {
            "--debug" => debug_mode = true,
            "--help" | "-h" => show_help = true,
            _ => {
                if file_path.is_none() && !args[i].starts_with('-') {
                    file_path = Some(args[i].clone());
                }
            }
        }
    }

    // 如果没有提供文件路径，显示用法信息并退出
    if file_path.is_none() {
        println!("用法: {} [--debug] [--help] <yaml文件路径>", args[0]);
        println!("选项:");
        println!("  --debug    启用调试模式");
        println!("  --help, -h 显示帮助视图");
        return Ok(());
    }

    let mut input = String::new();
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
    
    // 添加搜索相关状态
    let mut app_mode = AppMode::Normal;
    let mut search_query = String::new();
    let mut search_results: Vec<usize> = Vec::new();
    let mut current_match = 0;

    while !should_quit {
        terminal.draw(|f| {
            let size = f.size();
            let nodes = tree.flatten();
            let _max_y = (size.height - 1) as usize;

            // 先构建文本内容
            let text = nodes
                .iter()
                .enumerate()
                .map(|(i, node)| {
                    let indent = "  ".repeat(node.depth);
                    let content = match &node.value {
                        Value::Mapping(_) => {
                            if node.children.is_empty() {
                                format!("{{}}")
                            } else if node.expanded {
                                "".to_string()
                            } else {
                                format!("{{...}}")
                            }
                        }
                        Value::Sequence(_) => {
                            if node.children.is_empty() {
                                "[]".to_string()
                            } else if node.expanded {
                                "".to_string()
                            } else {
                                "[...]".to_string()
                            }
                        }
                        Value::Tagged(tagged) => {
                            serde_yaml::to_string(&tagged.value)
                                .unwrap()
                                .trim()
                                .to_string()
                        }
                        _ => {
                            serde_yaml::to_string(&node.value)
                                .unwrap()
                                .trim()
                                .to_string()
                        }
                    };

                    let full_text = if matches!(node.value, Value::Mapping(_) | Value::Sequence(_))
                        && node.expanded
                    {
                        format!("{}{}:", indent, node.key)
                    } else {
                        format!("{}{}: {}", indent, node.key, content)
                    };
                    let max_width = (size.width - 2) as usize; // Account for borders
                    let wrapped_text = textwrap::wrap(&full_text, max_width)
                        .into_iter()
                        .map(|line| {
                            // 高亮当前选中行
                            if i == cursor_pos {
                                Spans::from(vec![Span::styled(
                                    line.to_string(),
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                )])
                            } 
                            // 高亮搜索结果
                            else if !search_query.is_empty() && search_results.contains(&i) {
                                // 当前匹配项使用不同颜色
                                if search_results.get(current_match) == Some(&i) {
                                    Spans::from(vec![Span::styled(
                                        line.to_string(),
                                        Style::default()
                                            .fg(Color::Green)
                                            .add_modifier(Modifier::BOLD),
                                    )])
                                } else {
                                    Spans::from(vec![Span::styled(
                                        line.to_string(),
                                        Style::default()
                                            .fg(Color::Blue)
                                            .add_modifier(Modifier::BOLD),
                                    )])
                                }
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

            // 首先确定是否需要显示帮助视图
            let (main_area, help_area_opt) = if show_help {
                // 垂直分割为内容区域和帮助区域
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Min(5),       // Main content + debug (if enabled)
                        Constraint::Length(4),    // Help text
                    ])
                    .split(size);
                (chunks[0], Some(chunks[1]))
            } else {
                // 没有帮助视图，内容区域占据全部
                (size, None)
            };

            // 然后处理内容区域（可能包含调试视图）
            let content_area = if debug_mode {
                // 水平分割为内容和调试区域
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(70), // Main content
                        Constraint::Percentage(30), // Debug info
                    ])
                    .split(main_area);
                
                // 保存调试区域以供后续使用
                let debug_area = content_chunks[1];
                
                // 渲染主内容
                let main_panel = Paragraph::new(text.clone())
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(main_panel, content_chunks[0]);
                
                // 渲染调试面板
                if let Some(node) = nodes.get(cursor_pos) {
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

                    // 计算长度信息
                    let length_info = match &node.value {
                        Value::Sequence(seq) => Some(format!("{}", seq.len())),
                        Value::Mapping(map) => Some(format!("{}", map.len())),
                        _ => None,
                    };

                    let expanded_status =
                        if matches!(node.value, Value::Mapping(_) | Value::Sequence(_)) {
                            if node.expanded {
                                "Expanded"
                            } else {
                                "Collapsed"
                            }
                        } else {
                            "N/A"
                        };

                    let mut debug_spans = vec![
                        Spans::from(vec![
                            Span::raw("Path: "),
                            Span::styled(path, Style::default().fg(Color::Cyan)),
                        ]),
                        Spans::from(vec![
                            Span::raw("Type: "),
                            Span::styled(value_type, Style::default().fg(Color::Green)),
                        ]),
                    ];
                    
                    // 添加长度信息（如果有）
                    if let Some(length) = length_info {
                        debug_spans.push(Spans::from(vec![
                            Span::raw("Length: "),
                            Span::styled(length, Style::default().fg(Color::Magenta)),
                        ]));
                    }
                    
                    // 添加展开状态
                    debug_spans.push(Spans::from(vec![
                        Span::raw("Status: "),
                        Span::styled(expanded_status, Style::default().fg(Color::Yellow)),
                    ]));

                    let debug_info = Text::from(debug_spans);

                    let debug_panel = Paragraph::new(debug_info)
                        .block(Block::default().borders(Borders::ALL).title("Debug Info"));
                    
                    f.render_widget(debug_panel, debug_area);
                }
                
                content_chunks[0] // 返回主内容区域供搜索框使用
            } else {
                // 没有调试视图，直接渲染主内容
                let main_panel = Paragraph::new(text.clone())
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(main_panel, main_area);
                
                main_area // 返回主内容区域供搜索框使用
            };

            // 在搜索模式下显示搜索输入框在主视图底部
            if app_mode == AppMode::Search {
                // 计算搜索框的位置 - 放在主视图底部，但不与边框重叠
                let search_area = Rect {
                    x: content_area.x + 1, // 增加 x 坐标
                    y: content_area.y + content_area.height - 2, // 减少 y 坐标，避开底部边框
                    width: content_area.width - 4, // 减少宽度，避开右边框
                    height: 1,
                };
                
                // 渲染搜索文本
                let search_span = Spans::from(vec![
                    Span::styled("/", Style::default().fg(Color::Yellow)),
                    Span::raw(search_query.clone()),
                ]);
                
                let search_paragraph = Paragraph::new(search_span)
                    .style(Style::default());
                
                f.render_widget(search_paragraph, search_area);
                
                // 显示光标位置
                f.set_cursor(
                    search_area.x + search_query.len() as u16 + 1,
                    search_area.y,
                );
            }

            // 只有在启用帮助视图时才渲染帮助面板
            if let Some(help_area) = help_area_opt {
                // 根据当前模式显示不同的帮助信息
                let help_text = match app_mode {
                    AppMode::Normal => {
                        vec![
                            Spans::from(vec![
                                Span::styled("j/↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 下移  "),
                                Span::styled("k/↑", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 上移  "),
                                Span::styled("h", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 折叠节点  "),
                                Span::styled("l", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 展开节点  "),
                            ]),
                            Spans::from(vec![
                                Span::styled("/", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 搜索  "),
                                Span::styled("n", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 下一个匹配  "),
                                Span::styled("N", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 上一个匹配  "),
                                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 取消高亮  "),
                                Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 退出"),
                            ]),
                        ]
                    },
                    AppMode::Search => {
                        vec![
                            Spans::from(vec![
                                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 确认搜索  "),
                                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                                Span::raw(": 取消搜索"),
                            ]),
                        ]
                    },
                };
                
                let help_panel = Paragraph::new(help_text)
                    .style(Style::default())
                    .alignment(tui::layout::Alignment::Left);
                    
                f.render_widget(help_panel, help_area);
            }
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app_mode {
                        AppMode::Normal => {
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
                                    // Collapse the node
                                    process_node(&mut tree, cursor_pos, |node| node.expanded = false);
                                }
                                KeyCode::Char('l') => {
                                    // Expand the node
                                    process_node(&mut tree, cursor_pos, |node: &mut TreeNode| {
                                        node.expanded = true
                                    });
                                }
                                KeyCode::Char('/') => {
                                    // 进入搜索模式
                                    app_mode = AppMode::Search;
                                    search_query.clear();
                                }
                                KeyCode::Char('n') => {
                                    // 跳转到下一个搜索结果
                                    if !search_results.is_empty() {
                                        current_match = (current_match + 1) % search_results.len();
                                        if let Some(&pos) = search_results.get(current_match) {
                                            cursor_pos = pos;
                                        }
                                    }
                                }
                                KeyCode::Char('N') => {
                                    // 跳转到上一个搜索结果
                                    if !search_results.is_empty() {
                                        current_match = if current_match == 0 {
                                            search_results.len() - 1
                                        } else {
                                            current_match - 1
                                        };
                                        if let Some(&pos) = search_results.get(current_match) {
                                            cursor_pos = pos;
                                        }
                                    }
                                }
                                KeyCode::Esc => {
                                    // 在普通模式下，Esc 用于清除搜索高亮
                                    search_results.clear();
                                    search_query.clear();
                                }
                                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                    // Ctrl+C 也可以清除搜索高亮
                                    search_results.clear();
                                    search_query.clear();
                                }
                                _ => {}
                            }
                        }
                        AppMode::Search => {
                            match key.code {
                                KeyCode::Esc => {
                                    // 退出搜索模式
                                    app_mode = AppMode::Normal;
                                }
                                KeyCode::Enter => {
                                    // 执行搜索
                                    app_mode = AppMode::Normal;
                                    
                                    // 如果搜索查询不为空，执行搜索
                                    if !search_query.is_empty() {
                                        let query = search_query.to_lowercase();
                                        search_results.clear();
                                        
                                        // 搜索节点
                                        for (i, node) in tree.flatten().iter().enumerate() {
                                            let node_text = format!("{}", node.key).to_lowercase();
                                            if node_text.contains(&query) {
                                                search_results.push(i);
                                            }
                                            
                                            // 也搜索值
                                            if let Value::String(s) = &node.value {
                                                if s.to_lowercase().contains(&query) {
                                                    search_results.push(i);
                                                }
                                            }
                                        }
                                        
                                        // 如果有结果，跳转到第一个匹配项
                                        if !search_results.is_empty() {
                                            current_match = 0;
                                            cursor_pos = search_results[0];
                                        }
                                    }
                                }
                                KeyCode::Char(c) => {
                                    // 添加字符到搜索查询
                                    search_query.push(c);
                                }
                                KeyCode::Backspace => {
                                    // 删除字符
                                    search_query.pop();
                                }
                                _ => {}
                            }
                        }
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
