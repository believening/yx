# YAML TUI Viewer

一个用Rust编写的终端用户界面(TUI)应用程序，用于可视化浏览和操作YAML文件内容。

## 功能特性

- 🖥️ 终端界面展示YAML结构
- 🗂️ 支持展开/折叠对象节点
- 🎯 通过H/L键折叠/展开当前节点
- ⬆️⬇️ 通过J/K键上下移动光标
- 🎨 彩色语法高亮显示
- 🚀 实时交互响应

## AI协作开发

本项目完全由AI助手Cline开发完成，展示了AI在软件开发中的强大能力。开发过程包括：

1. 需求分析与架构设计
2. Rust代码实现
3. 生命周期问题调试
4. 功能迭代优化
5. 文档编写

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

## 使用说明

1. 安装Rust环境
2. 克隆项目仓库
3. 运行程序：
   ```bash
   cargo run --input test.yaml
   ```
4. 使用快捷键：
   - J/K: 上下移动
   - H/L: 折叠/展开
   - Q: 退出

## 开发过程总结

本项目展示了AI在软件开发中的完整工作流程：

1. 准确理解需求
2. 设计合理架构
3. 编写高质量代码
4. 解决复杂技术问题（如Rust生命周期）
5. 持续迭代优化
6. 编写完整文档

整个开发过程体现了AI作为开发助手的强大能力，能够高效完成从设计到实现的完整软件开发流程。

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
