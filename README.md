# YAML TUI Viewer

> **🤖 特别声明：本项目100%由AI助手编写完成，展示了人工智能在软件开发领域的实际应用。**

一个用Rust编写的终端用户界面(TUI)应用程序，用于可视化浏览和操作YAML文件内容。支持从文件或标准输入(stdin)读取YAML，轻松查看和探索复杂的YAML结构。

## 功能特性

- 🖥️ 直观的终端树状视图展示YAML结构
- 🔍 类似Vim的搜索功能，支持高亮和结果导航
- 🗂️ 交互式节点展开/折叠功能
- 🎨 彩色语法高亮显示不同类型的内容
- 📋 支持从文件或标准输入(stdin)读取YAML内容
- 🐞 可选的调试模式显示节点详细信息
- ℹ️ 内置帮助视图，易于上手
- 🚀 响应迅速，支持大型YAML文件

## 安装与使用

### 安装

1. 确保已安装Rust环境
2. 克隆并编译项目：
   ```bash
   git clone https://github.com/believening/yx.git
   cd yx
   cargo build --release
   ```
3. 编译后的可执行文件位于 `target/release/yaml-tui-viewer`

### 使用方法

#### 从文件读取
```bash
yaml-tui-viewer [选项] <yaml文件路径>

# 示例
yaml-tui-viewer config.yaml
yaml-tui-viewer --debug docker-compose.yml
```

#### 从管道读取
```bash
cat some.yaml | yaml-tui-viewer [选项]
curl -s https://api.example.com/data | yaml-tui-viewer
kubectl get pod my-pod -o yaml | yaml-tui-viewer --debug
```

#### 命令行选项

- `--debug`: 启用调试模式，显示节点的详细信息
- `--help` 或 `-h`: 显示内置帮助信息

## 交互指南

### 快捷键

| 类别 | 按键 | 功能 |
|------|------|------|
| **导航** | `j` 或 `↓` | 向下移动光标 |
|        | `k` 或 `↑` | 向上移动光标 |
|        | `h` | 折叠当前节点 |
|        | `l` | 展开当前节点 |
| **搜索** | `/` | 进入搜索模式 |
|        | `Enter` | 执行搜索 |
|        | `n` | 跳转到下一个搜索结果 |
|        | `N` | 跳转到上一个搜索结果 |
|        | `Esc` | 退出搜索模式/清除搜索高亮 |
| **其他** | `q` | 退出程序 |

### 搜索功能使用

搜索功能提供类似Vim的体验：
1. 按 `/` 键进入搜索模式
2. 输入搜索关键词
3. 按 `Enter` 执行搜索
4. 匹配项将使用不同颜色高亮显示
5. 使用 `n` 和 `N` 在匹配项之间跳转
6. 当前匹配项会有独特的高亮颜色

搜索功能会在节点名称和字符串值中查找匹配项。

## 注意事项

- 在使用管道输入时，程序会自动检测并使用 `/dev/tty` 获取键盘输入
- Windows系统下的管道支持可能受限，请优先使用文件读取方式
- 大型YAML文件可能会导致解析速度变慢
- 对于容器类型节点(对象和数组)支持折叠/展开，基本类型节点不支持此操作

## 项目结构

```
yaml-tui-viewer/
├── src/
│   └── main.rs        # 主程序文件
├── Cargo.toml         # 项目配置
├── Cargo.lock         # 依赖锁定
├── test.yaml          # 示例文件
└── README.md          # 项目文档
```

## AI开发说明

**📢 本项目是完全由AI执行编码完成的实例项目**，展示了AI在软件开发中的强大能力。开发过程包括：

1. 需求分析与架构设计
2. Rust代码实现
3. 生命周期问题调试
4. 功能迭代优化
5. 文档编写

整个开发过程体现了AI作为开发助手的能力，高效完成从设计到实现的完整软件开发流程。

### 关于需求描述的重要性

本项目的成功开发得益于最初清晰、详细的需求描述。准确的需求描述对于软件开发至关重要：

1. **明确功能边界**：详细的需求描述帮助开发者准确理解项目范围
2. **提高开发效率**：减少需求理解偏差，避免返工
3. **确保质量**：明确的功能描述有助于设计更完善的测试用例
4. **促进协作**：在AI辅助开发中，详细需求帮助AI更好地理解任务
5. **降低沟通成本**：减少开发过程中的需求澄清次数

本项目初始需求描述包含：
- 核心功能要求
- 具体交互细节
- 边界条件说明
- 输入输出规范

这种详细的需求描述为项目的顺利开发奠定了坚实基础。

以下是成功完成项目的初始需求描述:

```
使用RUST编写一个TUI软件，能够可视化展示读取到的 yaml 内容。通过 H 键折叠光标所在行节点内容，L 键展开光标所在行的节点内容，通过 J 和 K 键上下移动光标。

对于 value 为基本类型的节点，不支持折叠和展开。显示效果为 `key: value`。另外，对于 array 类型，如果 item 也是基本类型，也不支持展开折叠，显示效果为`- value`。

如果光标处于当前需要展示的内容的最后一行，则不支持向下移动光标。同样的，如果已在第一行，则不支持向上移动光标。

程序默认读取标准输入，并可通过 --input 支持读取文件输出。
```

## 贡献

欢迎提交问题和功能请求！如果您想贡献代码，请先开一个issue讨论您想要更改的内容。

## 许可证

MIT License

Copyright (c) 2023

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
