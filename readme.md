# Empty Folder Cleaner

查找并一键删除电脑中的空文件夹

A cross-platform desktop tool to find and clean up empty folders on your computer with ease.

---

## 核心特性 | Core Features

- **递归扫描**：深度遍历所选目录，精准定位所有空文件夹  
  **Recursive Scan**: Deeply traverses the selected directory to precisely locate all empty folders

- **安全删除**：支持永久删除或移动至回收站（需安装 `send2trash`），删除前二次确认  
  **Safe Deletion**: Supports permanent deletion or moving to trash (requires `send2trash`), with a confirmation dialog before deletion

- **智能忽略**：自动记住删除失败（权限不足）的文件夹，下次扫描不再显示；支持通配符规则忽略特定文件夹  
  **Smart Ignore**: Automatically remembers folders that failed to delete (e.g., permission denied) and hides them in future scans; supports wildcard ignore rules

- **现代化 GUI**：基于 PyQt6 的精美界面，支持暗色/明亮/跟随系统主题，界面简洁直观  
  **Modern GUI**: Beautiful PyQt6-based interface with dark / light / system theme switching, clean and intuitive

- **多语言支持**：内置中文和英文，可根据系统语言自动切换  
  **Multi‑language Support**: Built‑in Chinese and English, with automatic switching based on system language

- **便捷操作**：扫描进度实时显示，支持停止扫描/停止删除；全选/取消全选，单击勾选；排序与本地过滤  
  **Convenient Operation**: Real‑time scan progress with stop support; select all / deselect all, click to select; sort by path or modification time and local filtering

- **右键菜单**：打开文件夹位置、复制路径、忽略此文件夹  
  **Context Menu**: Open folder location, copy path, ignore this folder

- **历史目录**：自动记录最近扫描的目录，便于快速切换  
  **History Directories**: Automatically records recently scanned directories for quick switching

- **键盘快捷键**：`Ctrl+A` 全选，`F5` 刷新扫描，`Esc` 停止当前操作  
  **Keyboard Shortcuts**: `Ctrl+A` select all, `F5` refresh scan, `Esc` stop current operation

- **空间估算**：实时估算选中文件夹删除后可能释放的磁盘空间  
  **Space Estimation**: Real‑time estimation of disk space that could be freed after deleting selected folders

---

## 使用说明 | Usage Guide

1. 下载并运行本程序（Windows 用户可直接运行 `.exe`，macOS / Linux 需安装 Python 依赖）  
   Download and run the program (Windows users can directly run the `.exe`; macOS / Linux requires Python dependencies)

2. 选择要扫描的目录（默认用户主目录），或从历史目录下拉框中快速选择  
   Select the directory to scan (default is your home directory), or quickly pick one from the recent directories dropdown

3. 程序自动开始扫描，扫描结果实时出现在表格中  
   The program starts scanning automatically, and results appear in the table in real time

4. 在列表中勾选需要删除的空文件夹；可使用搜索框快速筛选  
   Check the empty folders you want to delete; use the search box to filter the list

5. 点击“删除选中”，确认后即可删除（或移至回收站）；删除失败的文件夹会自动被忽略，下次扫描不再显示  
   Click “Delete Selected”, confirm, and the folders will be deleted (or moved to trash); any folder that fails to delete will be automatically ignored in future scans

6. 可通过“管理忽略规则”设置通配符规则（如 `*cache*`），匹配的文件夹将不在扫描结果中出现  
   You can set wildcard ignore rules (e.g., `*cache*`) via “Manage Ignore Rules”; matched folders will be excluded from scans

---

## 安装依赖 | Dependencies

- Python 3.8+
- PyQt6
- darkdetect（可选，用于跟随系统主题）
- send2trash（可选，用于安全删除至回收站）

## 项目贡献者 | Contributors

| 贡献者 (Contributor) | 贡献内容 (Contribution) |
|----------------------|--------------------------|
| 用户 (User)          | 完整开发 (Complete development) |

*(欢迎提交 PR 加入贡献者列表)*  
*(Welcome to submit PR to join the contributor list)*

---

## 许可协议 | License

本项目采用 MIT 许可证，详情参见 `LICENSE` 文件。  
This project is licensed under the MIT License, see the `LICENSE` file for details.

---

## 支持我们 | Support Us

如果这个项目对您有帮助，欢迎点亮右上角的 Star ⭐ 支持我们，这将是对所有贡献者最大的鼓励！  
If this project is helpful to you, please feel free to star it in the upper right corner ⭐ to support us, which will be the greatest encouragement to all contributors!
