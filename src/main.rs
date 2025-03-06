use termion::{
    event::{Event, Key},
    raw::IntoRawMode,
    screen::IntoAlternateScreen,
};
use serde_yaml::Value;
use std::{
    env, fs,
    io::{self, Read},
    path::Path,
    fs::File,
    error::Error,
};
use ratatui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

// 常量定义，使样式和配置更易于管理
const HIGHLIGHT_STYLE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const CURRENT_MATCH_STYLE: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const MATCH_STYLE: Style = Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD);

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
                if Self::is_container(&child.value) && child.expanded {
                    let mut child_nodes = child.flatten();
                    result.append(&mut child_nodes);
                } else {
                    result.push(child);
                }
            }
        }
        result
    }

    // 提取判断是否为容器类型的逻辑为函数
    fn is_container(value: &Value) -> bool {
        matches!(value, Value::Mapping(_) | Value::Sequence(_))
    }

    // 添加获取格式化内容的函数
    fn get_formatted_content(&self) -> String {
        match &self.value {
            Value::Mapping(_) => {
                if self.children.is_empty() {
                    "{}".to_string()
                } else if self.expanded {
                    "".to_string()
                } else {
                    "{...}".to_string()
                }
            }
            Value::Sequence(_) => {
                if self.children.is_empty() {
                    "[]".to_string()
                } else if self.expanded {
                    "".to_string()
                } else {
                    "[...]".to_string()
                }
            }
            Value::Tagged(tagged) => {
                serde_yaml::to_string(&tagged.value)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }
            _ => {
                serde_yaml::to_string(&self.value)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }
        }
    }

    // 添加获取完整显示文本的函数
    fn get_display_text(&self) -> String {
        let indent = "  ".repeat(self.depth);

        if Self::is_container(&self.value) && self.expanded {
            format!("{}{}:", indent, self.key)
        } else {
            format!("{}{}: {}", indent, self.key, self.get_formatted_content())
        }
    }
}

// 自定义错误类型，便于错误处理
#[derive(Debug)]
enum AppError {
    Io(io::Error),
    YamlParse(serde_yaml::Error),
    EmptyInput,
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        AppError::YamlParse(err)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "IO错误: {}", err),
            AppError::YamlParse(err) => write!(f, "YAML解析错误: {}", err),
            AppError::EmptyInput => write!(f, "输入的YAML内容为空"),
        }
    }
}

impl Error for AppError {}

fn build_yaml_tree(input: &str) -> Result<TreeNode, AppError> {
    let value: Value = serde_yaml::from_str(input)?;
    Ok(TreeNode::new("root".to_string(), value, 0))
}

fn process_node(tree: &mut TreeNode, cursor_pos: usize, mut action: impl FnMut(&mut TreeNode)) {
    let nodes = tree.flatten();
    if let Some(node) = nodes.get(cursor_pos) {
        if TreeNode::is_container(&node.value) {
            // 构建路径
            let mut path = vec![];
            let mut current = node;
            while let Some(parent) = nodes.iter().find(|n| n.children.contains(current)) {
                path.push(parent.key.clone());
                current = parent;
            }
            path.reverse();
            path.push(node.key.clone());

            // 遍历树找到目标节点
            let mut target_node = Some(tree);
            for key in path.iter().skip(1) {
                if let Some(current_mut) = target_node {
                    target_node = current_mut
                        .children
                        .iter_mut()
                        .find(|child| &child.key == key);
                } else {
                    break;
                }
            }

            // 应用操作
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

// 应用状态结构体
struct AppState {
    tree: TreeNode,
    cursor_pos: usize,
    app_mode: AppMode,
    search_query: String,
    search_results: Vec<usize>,
    current_match: usize,
    debug_mode: bool,
    show_help: bool,
}

impl AppState {
    fn new(tree: TreeNode, debug_mode: bool, show_help: bool) -> Self {
        let mut root = tree;
        root.expanded = true;

        Self {
            tree: root,
            cursor_pos: 0,
            app_mode: AppMode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            current_match: 0,
            debug_mode,
            show_help,
        }
    }

    // 处理正常模式下的按键
    fn handle_normal_mode_key(&mut self, key: Key) -> bool {
        let mut should_quit = false;

        match key {
            Key::Char('q') => should_quit = true,
            Key::Down | Key::Char('j') => self.move_cursor_down(),
            Key::Up | Key::Char('k') => self.move_cursor_up(),
            Key::Char('h') => self.collapse_node(),
            Key::Char('l') => self.expand_node(),
            Key::Char('/') => self.enter_search_mode(),
            Key::Char('n') => self.next_search_match(),
            Key::Char('N') => self.prev_search_match(),
            Key::Esc => self.clear_search(),
            _ => {}
        }

        should_quit
    }

    // 处理搜索模式下的按键
    fn handle_search_mode_key(&mut self, key: Key) {
        match key {
            Key::Esc => self.exit_search_mode(),
            Key::Char('\n') => self.perform_search(),
            Key::Char(c) => self.search_query.push(c),
            Key::Backspace => { self.search_query.pop(); },
            _ => {}
        }
    }

    fn move_cursor_down(&mut self) {
        let total_lines = self.tree.flatten().len();
        if self.cursor_pos < total_lines - 1 {
            self.cursor_pos += 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    fn collapse_node(&mut self) {
        process_node(&mut self.tree, self.cursor_pos, |node| node.expanded = false);
    }

    fn expand_node(&mut self) {
        process_node(&mut self.tree, self.cursor_pos, |node| node.expanded = true);
    }

    fn enter_search_mode(&mut self) {
        self.app_mode = AppMode::Search;
        self.search_query.clear();
    }

    fn exit_search_mode(&mut self) {
        self.app_mode = AppMode::Normal;
    }

    fn clear_search(&mut self) {
        self.search_results.clear();
        self.search_query.clear();
    }

    fn next_search_match(&mut self) {
        if !self.search_results.is_empty() {
            self.current_match = (self.current_match + 1) % self.search_results.len();
            if let Some(&pos) = self.search_results.get(self.current_match) {
                self.cursor_pos = pos;
            }
        }
    }

    fn prev_search_match(&mut self) {
        if !self.search_results.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.search_results.len() - 1
            } else {
                self.current_match - 1
            };
            if let Some(&pos) = self.search_results.get(self.current_match) {
                self.cursor_pos = pos;
            }
        }
    }

    fn perform_search(&mut self) {
        self.app_mode = AppMode::Normal;

        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            self.search_results.clear();

            for (i, node) in self.tree.flatten().iter().enumerate() {
                let node_text = node.key.to_lowercase();
                if node_text.contains(&query) {
                    self.search_results.push(i);
                }

                if let Value::String(s) = &node.value {
                    if s.to_lowercase().contains(&query) {
                        self.search_results.push(i);
                    }
                }
            }

            if !self.search_results.is_empty() {
                self.current_match = 0;
                self.cursor_pos = self.search_results[0];
            }
        }
    }
}

// UI 渲染函数，将复杂的渲染逻辑分离出来
fn render_ui(
    f: &mut ratatui::Frame,
    state: &AppState,
) {
    let size = f.size();
    let nodes = state.tree.flatten();

    // 构建文本内容
    let text = build_text_content(&nodes, state);

    // 布局管理
    let (main_area, help_area_opt) = if state.show_help {
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

    // 处理内容区域（可能包含调试视图）
    let content_area = if state.debug_mode {
        render_with_debug(f, main_area, &text, &nodes, state.cursor_pos)
    } else {
        render_main_content(f, main_area, &text)
    };

    // 在搜索模式下显示搜索输入框
    if state.app_mode == AppMode::Search {
        render_search_box(f, content_area, &state.search_query);
    }

    // 渲染帮助面板
    if let Some(help_area) = help_area_opt {
        render_help_panel(f, help_area, &state.app_mode);
    }
}

// 构建文本内容函数
fn build_text_content<'a>(
    nodes: &Vec<&'a TreeNode>,
    state: &AppState
) -> Vec<Line<'a>> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let full_text = node.get_display_text();
            let max_width = 100; // 假设一个合理的最大宽度

            let wrapped_text = textwrap::wrap(&full_text, max_width)
                .into_iter()
                .map(|line| {
                    // 应用样式逻辑
                    if i == state.cursor_pos {
                        Line::from(vec![Span::styled(line.to_string(), HIGHLIGHT_STYLE)])
                    }
                    else if !state.search_query.is_empty() && state.search_results.contains(&i) {
                        if state.search_results.get(state.current_match) == Some(&i) {
                            Line::from(vec![Span::styled(line.to_string(), CURRENT_MATCH_STYLE)])
                        } else {
                            Line::from(vec![Span::styled(line.to_string(), MATCH_STYLE)])
                        }
                    } else {
                        Line::from(vec![Span::raw(line.to_string())])
                    }
                })
                .collect::<Vec<_>>();

            // 处理换行
            let wrapped_len = wrapped_text.len();
            let mut result = Vec::new();
            for (line_idx, line) in wrapped_text.into_iter().enumerate() {
                result.push(line);
                if line_idx < wrapped_len - 1 {
                    result.push(Line::from(vec![Span::raw("")]));
                }
            }
            result
        })
        .flatten()
        .collect()
}

// 渲染主内容
fn render_main_content(
    f: &mut ratatui::Frame,
    area: Rect,
    text: &[Line],
) -> Rect {
    let main_panel = Paragraph::new(text.to_vec())
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(main_panel, area);
    area
}

// 渲染带调试视图的内容
fn render_with_debug(
    f: &mut ratatui::Frame,
    area: Rect,
    text: &[Line],
    nodes: &Vec<&TreeNode>,
    cursor_pos: usize,
) -> Rect {
    // 水平分割为内容和调试区域
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Main content
            Constraint::Percentage(30), // Debug info
        ])
        .split(area);

    // 渲染主内容
    let main_panel = Paragraph::new(text.to_vec())
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(main_panel, content_chunks[0]);

    // 渲染调试面板
    if let Some(node) = nodes.get(cursor_pos) {
        render_debug_panel(f, content_chunks[1], node, nodes);
    }

    content_chunks[0]
}

// 渲染调试面板
fn render_debug_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    node: &TreeNode,
    nodes: &[&TreeNode],
) {
    // 构建路径
    let mut path = vec![];
    let mut current = node;
    while let Some(parent) = nodes.iter().find(|n| n.children.contains(current)) {
        path.push(parent.key.clone());
        current = parent;
    }
    path.reverse();
    path.push(node.key.clone());
    let path = path.join(" > ");

    // 获取节点类型
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

    // 获取展开状态
    let expanded_status = if TreeNode::is_container(&node.value) {
        if node.expanded { "Expanded" } else { "Collapsed" }
    } else {
        "N/A"
    };

    // 构建调试信息
    let mut debug_spans = vec![
        Line::from(vec![
            Span::raw("Path: "),
            Span::styled(path, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::raw("Type: "),
            Span::styled(value_type, Style::default().fg(Color::Green)),
        ]),
    ];

    // 添加长度信息（如果有）
    if let Some(length) = length_info {
        debug_spans.push(Line::from(vec![
            Span::raw("Length: "),
            Span::styled(length, Style::default().fg(Color::Magenta)),
        ]));
    }

    // 添加展开状态
    debug_spans.push(Line::from(vec![
        Span::raw("Status: "),
        Span::styled(expanded_status, Style::default().fg(Color::Yellow)),
    ]));

    let debug_info = Text::from(debug_spans);
    let debug_panel = Paragraph::new(debug_info)
        .block(Block::default().borders(Borders::ALL).title("Debug Info"));

    f.render_widget(debug_panel, area);
}

// 渲染搜索框
fn render_search_box(
    f: &mut ratatui::Frame,
    content_area: Rect,
    search_query: &str,
) {
    let search_area = Rect {
        x: content_area.x + 1,
        y: content_area.y + content_area.height - 2,
        width: content_area.width - 4,
        height: 1,
    };

    let search_span = Line::from(vec![
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(search_query),
    ]);

    let search_paragraph = Paragraph::new(vec![search_span])
        .style(Style::default());

    f.render_widget(search_paragraph, search_area);

    // 显示光标位置
    f.set_cursor(
        search_area.x + search_query.len() as u16 + 1,
        search_area.y,
    );
}

// 渲染帮助面板
fn render_help_panel(
    f: &mut ratatui::Frame,
    area: Rect,
    app_mode: &AppMode,
) {
    let help_text = match app_mode {
        AppMode::Normal => {
            vec![
                Line::from(vec![
                    Span::styled("j/↓", HIGHLIGHT_STYLE),
                    Span::raw(": 下移  "),
                    Span::styled("k/↑", HIGHLIGHT_STYLE),
                    Span::raw(": 上移  "),
                    Span::styled("h", HIGHLIGHT_STYLE),
                    Span::raw(": 折叠节点  "),
                    Span::styled("l", HIGHLIGHT_STYLE),
                    Span::raw(": 展开节点  "),
                ]),
                Line::from(vec![
                    Span::styled("/", HIGHLIGHT_STYLE),
                    Span::raw(": 搜索  "),
                    Span::styled("n", HIGHLIGHT_STYLE),
                    Span::raw(": 下一个匹配  "),
                    Span::styled("N", HIGHLIGHT_STYLE),
                    Span::raw(": 上一个匹配  "),
                    Span::styled("Esc", HIGHLIGHT_STYLE),
                    Span::raw(": 取消高亮  "),
                    Span::styled("q", HIGHLIGHT_STYLE),
                    Span::raw(": 退出"),
                ]),
            ]
        },
        AppMode::Search => {
            vec![
                Line::from(vec![
                    Span::styled("Enter", HIGHLIGHT_STYLE),
                    Span::raw(": 确认搜索  "),
                    Span::styled("Esc", HIGHLIGHT_STYLE),
                    Span::raw(": 取消搜索"),
                ]),
            ]
        },
    };

    let help_panel = Paragraph::new(help_text)
        .style(Style::default())
        .alignment(ratatui::layout::Alignment::Left);

    f.render_widget(help_panel, area);
}

// 读取输入（从文件或标准输入）
fn read_input(file_path: Option<String>) -> Result<String, AppError> {
    let input = if let Some(path) = file_path {
        // 从文件读取
        fs::read_to_string(Path::new(&path))?
    } else {
        // 检查stdin是否来自管道
        let stdin_is_pipe = !atty::is(atty::Stream::Stdin);

        if stdin_is_pipe {
            // 从管道读取
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        } else {
            // 没有提供文件路径且没有管道输入
            return Err(AppError::EmptyInput);
        }
    };

    // 检查输入是否为空
    if input.trim().is_empty() {
        return Err(AppError::EmptyInput);
    }

    Ok(input)
}

// 获取终端事件输入源（跨平台支持）
fn get_input_source() -> Result<Box<dyn Read>, io::Error> {
    if atty::is(atty::Stream::Stdin) {
        // 如果stdin是终端，直接使用stdin
        Ok(Box::new(io::stdin()))
    } else {
        // 如果stdin是管道，使用/dev/tty（Unix系统）或尝试其他方法（Windows系统）
        #[cfg(unix)]
        {
            Ok(Box::new(File::open("/dev/tty")?))
        }
        #[cfg(not(unix))]
        {
            // Windows系统下的替代方案
            // 注意：Windows下可能需要使用不同的方法获取控制台输入
            // 这里使用标准输入作为后备，但可能在管道重定向情况下不工作
            eprintln!("警告：在非Unix系统上，管道输入模式下的键盘交互可能无法正常工作");
            Ok(Box::new(io::stdin()))
        }
    }
}

fn run_app() -> Result<(), AppError> {
    // 解析命令行参数
    let args: Vec<String> = env::args().collect();
    let (debug_mode, show_help, file_path) = parse_args(&args);

    // 读取YAML内容
    let input = match read_input(file_path) {
        Ok(content) => content,
        Err(AppError::EmptyInput) => {
            show_usage(&args[0]);
            return Ok(());
        },
        Err(e) => return Err(e),
    };

    // 解析YAML并构建树
    let tree = build_yaml_tree(&input)?;

    // 初始化应用状态
    let mut app_state = AppState::new(tree, debug_mode, show_help);

    // 初始化终端
    let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建事件读取器
    let input = get_input_source()?;
    let mut events = termion::input::TermRead::events(input);

    let mut should_quit = false;

    // 主循环
    while !should_quit {
        terminal.draw(|f| render_ui(f, &app_state))?;

        // 使用 termion 的事件处理
        if let Some(Ok(event)) = events.next() {
            match event {
                Event::Key(key) => {
                    match app_state.app_mode {
                        AppMode::Normal => {
                            should_quit = app_state.handle_normal_mode_key(key);
                        }
                        AppMode::Search => {
                            app_state.handle_search_mode_key(key);
                        }
                    }
                },
                _ => {}
            }
        }
    }

    Ok(())
}

// 命令行参数解析
fn parse_args(args: &[String]) -> (bool, bool, Option<String>) {
    let mut debug_mode = false;
    let mut show_help = false;
    let mut file_path = None;

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

    (debug_mode, show_help, file_path)
}

// 显示用法信息
fn show_usage(program_name: &str) {
    println!("用法: {} [--debug] [--help] [<yaml文件路径>]", program_name);
    println!("或者: cat some.yaml | {} [--debug] [--help]", program_name);
    println!("选项:");
    println!("  --debug    启用调试模式");
    println!("  --help, -h 显示帮助视图");
}

fn main() {
    // 使用更优雅的错误处理
    if let Err(e) = run_app() {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
